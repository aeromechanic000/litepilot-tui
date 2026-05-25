use crate::agent::syntax::SyntaxChecker;
use crate::agent::{AgentPipeline, FileChange};
use crate::app::AppMode;
use crate::codebase::CodeBase;
use crate::config::Config;
use crate::ollama::OllamaClient;
use crate::project::file_ops::FileOps;
use crate::sandbox::Sandbox;
use anyhow::Result;
use std::path::PathBuf;

#[allow(dead_code)]
const MAX_RETRIES: usize = 2;

#[allow(dead_code)]
pub async fn run_auto_pipeline(
    client: &OllamaClient,
    config: &Config,
    sandbox: &Sandbox,
    workspace: PathBuf,
    user_request: &str,
    project_context: &str,
    codebase: Option<&CodeBase>,
) -> Result<Vec<FileChange>> {
    let pipeline = AgentPipeline::new(client, config, sandbox, workspace.clone());
    let file_ops = FileOps::new(sandbox, AppMode::Auto);

    // Step 1: Plan (with template retrieval if codebase available)
    let (plan, template_refs) = if let Some(cb) = codebase {
        let plan = pipeline
            .plan_with_templates(user_request, project_context, cb)
            .await?;
        let loaded =
            crate::codebase::retrieval::retrieve(client, config, cb, user_request, project_context)
                .await;
        (plan, loaded.refs)
    } else {
        let plan = pipeline.plan(user_request, project_context).await?;
        (plan, Vec::new())
    };

    // Step 2: Implement + Audit loop
    let mut attempts = 0;
    let mut final_changes = Vec::new();

    while attempts <= MAX_RETRIES {
        let changes = pipeline
            .implement(&plan, project_context, &template_refs)
            .await?;

        if changes.is_empty() {
            break;
        }

        // Step 3: Audit
        let audit = pipeline.audit(&changes, project_context).await?;

        if audit.passed {
            // Apply all changes
            for change in &changes {
                let fc = file_ops.prepare_write(&change.path, &change.content)?;
                file_ops.apply_change(&fc)?;
            }

            // Run syntax checks on all applied files
            let mut syntax_errors = Vec::new();
            for change in &changes {
                let full_path = workspace.join(&change.path);
                if let Ok(crate::agent::syntax::SyntaxResult::Fail { errors }) =
                    SyntaxChecker::check(&full_path, sandbox).await
                {
                    syntax_errors.push((change.path.to_string_lossy().to_string(), errors));
                }
            }

            if !syntax_errors.is_empty() {
                // Feed syntax errors back for correction on next attempt
                let _error_summary: String = syntax_errors
                    .iter()
                    .map(|(path, err)| format!("{}:\n{}", path, err))
                    .collect::<Vec<_>>()
                    .join("\n\n");

                attempts += 1;
                if attempts <= MAX_RETRIES {
                    // TODO: feed error_summary back into implement step
                    // For now, just report the errors and keep the applied files
                }
            }

            final_changes = changes;
            break;
        }

        attempts += 1;
        if attempts > MAX_RETRIES {
            final_changes = changes;
        }
    }

    Ok(final_changes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn max_retries_constant() {
        assert_eq!(super::MAX_RETRIES, 2);
    }
}
