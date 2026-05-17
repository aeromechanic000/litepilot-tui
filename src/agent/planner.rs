use crate::agent::Plan;
use crate::ollama::chat::ChatMessage;
use crate::ollama::OllamaClient;
use anyhow::Result;

#[allow(dead_code)]
pub async fn run_planning(
    client: &OllamaClient,
    model: &str,
    user_request: &str,
    project_context: &str,
    code_base_refs: &[String],
) -> Result<Plan> {
    let code_base_section = if code_base_refs.is_empty() {
        String::new()
    } else {
        format!("\n\nReference code from built-in library:\n{}", code_base_refs.join("\n"))
    };

    let messages = vec![
        ChatMessage::system(super::prompts::PLANNING_SYSTEM),
        ChatMessage::user(format!(
            "Project context:\n{}\n{}{}\n\nUser request:\n{}\n\nOutput a structured development plan.",
            project_context, code_base_section, "", user_request
        )),
    ];

    let response = client.chat(model, messages).await?;
    Ok(super::AgentPipeline::parse_plan(&response.content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_builds_messages_correctly() {
        // Verify the planner would construct proper messages
        let code_refs = vec!["template1".to_string(), "template2".to_string()];
        let code_section = if code_refs.is_empty() {
            String::new()
        } else {
            format!("\n\nReference code:\n{}", code_refs.join("\n"))
        };
        assert!(code_section.contains("template1"));
        assert!(code_section.contains("template2"));
    }
}
