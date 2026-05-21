use crate::agent::FileChange;
use crate::ollama::chat::ChatMessage;
use crate::ollama::OllamaClient;
use anyhow::Result;

#[allow(dead_code)]
pub async fn run_implementation(
    client: &OllamaClient,
    model: &str,
    plan_steps: &[String],
    project_context: &str,
) -> Result<Vec<FileChange>> {
    let messages = vec![
        ChatMessage::system(super::prompts::CODING_SYSTEM),
        ChatMessage::user(format!(
            "Project context:\n{}\n\nImplementation steps:\n{}\n\nImplement all changes. Format:\n### FILE: path\n### ACTION: create|modify|delete\n```code```",
            project_context,
            plan_steps.join("\n")
        )),
    ];

    let response = client.chat(model, messages, true).await?;
    Ok(super::AgentPipeline::parse_file_changes(&response.content))
}

#[cfg(test)]
mod tests {
    // Integration tests for editor module would go here
    // requiring a live Ollama instance
}
