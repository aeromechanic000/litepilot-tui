pub mod auto_run;
pub mod editor;
pub mod planner;
pub mod prompts;
pub mod retry;
pub mod syntax;

use crate::codebase::CodeBase;
use crate::config::Config;
use crate::ollama::chat::ChatMessage;
use crate::ollama::OllamaClient;
use crate::sandbox::Sandbox;
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Plan {
    pub analysis: String,
    pub steps: Vec<String>,
    pub files: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FileChange {
    pub path: PathBuf,
    pub content: String,
    pub action: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuditResult {
    pub passed: bool,
    pub issues: Vec<String>,
    pub fixes: Vec<FileChange>,
}

#[allow(dead_code)]
pub struct AgentPipeline<'a> {
    client: &'a OllamaClient,
    config: &'a Config,
    sandbox: &'a Sandbox,
    workspace: PathBuf,
}

#[allow(dead_code)]
impl<'a> AgentPipeline<'a> {
    pub fn new(
        client: &'a OllamaClient,
        config: &'a Config,
        sandbox: &'a Sandbox,
        workspace: PathBuf,
    ) -> Self {
        Self {
            client,
            config,
            sandbox,
            workspace,
        }
    }

    pub async fn plan(&self, user_request: &str, context: &str) -> Result<Plan> {
        let messages = vec![
            ChatMessage::system(prompts::PLANNING_SYSTEM),
            ChatMessage::user(format!(
                "Project context:\n{}\n\nUser request:\n{}\n\nOutput a structured plan with steps, files to create/modify, and dependencies.",
                context, user_request
            )),
        ];
        let model = self.config.effective_fast_model();
        let response = self.client.chat(model, messages, true).await?;
        Ok(Self::parse_plan(&response.content))
    }

    pub async fn plan_with_templates(
        &self,
        user_request: &str,
        context: &str,
        codebase: &CodeBase,
    ) -> Result<Plan> {
        let loaded = crate::codebase::retrieval::retrieve(
            self.client,
            self.config,
            codebase,
            user_request,
            context,
        )
        .await;

        let code_base_section = if loaded.refs.is_empty() {
            String::new()
        } else {
            format!(
                "\n\nReference code from built-in library:\n{}",
                loaded.refs.join("\n")
            )
        };

        let messages = vec![
            ChatMessage::system(prompts::PLANNING_SYSTEM),
            ChatMessage::user(format!(
                "Project context:\n{}{}\n\nUser request:\n{}\n\nOutput a structured plan with steps, files to create/modify, and dependencies.",
                context, code_base_section, user_request
            )),
        ];
        let model = self.config.effective_fast_model();
        let response = self.client.chat(model, messages, true).await?;
        Ok(Self::parse_plan(&response.content))
    }

    pub async fn implement(
        &self,
        plan: &Plan,
        context: &str,
        template_refs: &[String],
    ) -> Result<Vec<FileChange>> {
        let code_base_section = if template_refs.is_empty() {
            String::new()
        } else {
            format!(
                "\n\nReference code from built-in library:\n{}",
                template_refs.join("\n")
            )
        };

        let messages = vec![
            ChatMessage::system(prompts::CODING_SYSTEM),
            ChatMessage::user(format!(
                "Project context:\n{}{}\n\nPlan:\n{}\n\nImplement all changes. For each file, output:\n### FILE: path/to/file\n### ACTION: create|modify|delete\n```\nfile content\n```\n",
                context, code_base_section,
                plan.steps.join("\n")
            )),
        ];
        let model = &self.config.core_model;
        let response = self.client.chat(model, messages, true).await?;
        Ok(Self::parse_file_changes(&response.content))
    }

    pub async fn audit(&self, changes: &[FileChange], context: &str) -> Result<AuditResult> {
        let changes_str: Vec<String> = changes
            .iter()
            .map(|c| format!("--- {} ({}) ---\n{}", c.path.display(), c.action, c.content))
            .collect();
        let messages = vec![
            ChatMessage::system(prompts::AUDIT_SYSTEM),
            ChatMessage::user(format!(
                "Project context:\n{}\n\nChanges to review:\n{}\n\nCheck for bugs, logic errors, and consistency issues. Output PASS or FAIL followed by any issues.",
                context,
                changes_str.join("\n\n")
            )),
        ];
        let model = self.config.effective_audit_model();
        let response = self.client.chat(model, messages, true).await?;
        Ok(Self::parse_audit(&response.content))
    }

    fn parse_plan(text: &str) -> Plan {
        Plan {
            analysis: text.to_string(),
            steps: text
                .lines()
                .filter(|l| {
                    l.trim().starts_with("- ")
                        || l.trim().starts_with("* ")
                        || l.trim().chars().next().is_some_and(|c| c.is_ascii_digit())
                })
                .map(|l| l.trim().to_string())
                .collect(),
            files: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    pub fn parse_file_changes(text: &str) -> Vec<FileChange> {
        let mut changes = Vec::new();
        let mut current_path = String::new();
        let mut current_action = String::new();
        let mut current_content = String::new();
        let mut in_code_block = false;

        for line in text.lines() {
            if line.starts_with("### FILE:") {
                if !current_path.is_empty() && !current_content.is_empty() {
                    changes.push(FileChange {
                        path: PathBuf::from(&current_path),
                        content: current_content.trim_end().to_string(),
                        action: current_action.clone(),
                    });
                }
                current_path = line.trim_start_matches("### FILE:").trim().to_string();
                current_content.clear();
                in_code_block = false;
            } else if line.starts_with("### ACTION:") {
                current_action = line.trim_start_matches("### ACTION:").trim().to_string();
            } else if line.trim() == "```" {
                in_code_block = !in_code_block;
            } else if in_code_block {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        if !current_path.is_empty() && !current_content.is_empty() {
            changes.push(FileChange {
                path: PathBuf::from(&current_path),
                content: current_content.trim_end().to_string(),
                action: current_action,
            });
        }

        changes
    }

    fn parse_audit(text: &str) -> AuditResult {
        let upper = text.to_uppercase();
        let passed = upper.contains("PASS") && !upper.contains("FAIL");
        let issues: Vec<String> = text
            .lines()
            .filter(|l| {
                let lower = l.to_lowercase();
                lower.contains("error")
                    || lower.contains("bug")
                    || lower.contains("issue")
                    || lower.contains("problem")
            })
            .map(|l| l.to_string())
            .collect();
        AuditResult {
            passed,
            issues,
            fixes: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plan_extracts_steps() {
        let text =
            "# Plan\n\nSteps:\n- Create main.rs\n- Add config module\n- Write tests\n1. First do X";
        let plan = AgentPipeline::parse_plan(&text);
        assert!(!plan.steps.is_empty());
        assert!(plan.analysis.contains("Plan"));
    }

    #[test]
    fn parse_file_changes_extracts_blocks() {
        let text = r#"### FILE: src/main.rs
### ACTION: create
```
fn main() {
    println!("hello");
}
```

### FILE: src/lib.rs
### ACTION: create
```
pub fn add(a: i32, b: i32) -> i32 { a + b }
```
"#;
        let changes = AgentPipeline::parse_file_changes(&text);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].path, PathBuf::from("src/main.rs"));
        assert!(changes[0].content.contains("hello"));
        assert_eq!(changes[1].path, PathBuf::from("src/lib.rs"));
    }

    #[test]
    fn parse_audit_pass() {
        let text = "Review: PASS\nCode looks good, clean implementation.";
        let result = AgentPipeline::parse_audit(text);
        assert!(result.passed);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn parse_audit_fail() {
        let text = "Review: FAIL\nIssue: Missing error handling on line 42\nBug: potential null dereference";
        let result = AgentPipeline::parse_audit(text);
        assert!(!result.passed);
        assert!(!result.issues.is_empty());
    }

    #[test]
    fn parse_empty_changes() {
        let changes = AgentPipeline::parse_file_changes("");
        assert!(changes.is_empty());
    }
}
