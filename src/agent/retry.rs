use crate::ollama::chat::ChatMessage;
use crate::ollama::OllamaClient;

/// What kind of response we expect — determines how to validate it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResponseKind {
    /// Freeform text — just needs to be non-empty
    Chat,
    /// Code implementation — should contain FILE/ACTION blocks
    CodeImplementation,
}

/// Result of a validated response attempt.
pub enum ValidationResult {
    Valid(String),
    Invalid { response: String, reason: String },
}

/// Validates a model response based on the expected kind.
pub fn validate_response(content: &str, kind: ResponseKind) -> ValidationResult {
    let trimmed = content.trim();

    if trimmed.is_empty() {
        return ValidationResult::Invalid {
            response: content.to_string(),
            reason: "Response is empty. The model produced no output.".into(),
        };
    }

    match kind {
        ResponseKind::Chat => ValidationResult::Valid(content.to_string()),
        ResponseKind::CodeImplementation => validate_code_response(trimmed),
    }
}

fn validate_code_response(content: &str) -> ValidationResult {
    let has_file_marker = content.contains("### FILE:");
    let has_code_block = content.contains("```");

    if !has_file_marker && !has_code_block {
        return ValidationResult::Invalid {
            response: content.to_string(),
            reason: "Response does not contain any file changes. \
                     Expected format:\n\
                     ### FILE: path/to/file\n\
                     ### ACTION: create|modify|delete\n\
                     ```\ncode here\n```\n\n\
                     Please output the complete file contents using this exact format."
                .into(),
        };
    }

    if has_file_marker && !has_code_block {
        return ValidationResult::Invalid {
            response: content.to_string(),
            reason: "Found file markers (### FILE:) but no code blocks (```). \
                     Each file must include its content inside ``` blocks."
                .into(),
        };
    }

    // Check for unclosed code blocks
    let fence_count = content.matches("```").count();
    if !fence_count.is_multiple_of(2) {
        return ValidationResult::Invalid {
            response: content.to_string(),
            reason: "Unclosed code block detected (odd number of ``` fences). \
                     Make sure every opening ``` has a matching closing ```."
                .into(),
        };
    }

    // Check that FILE markers have ACTION markers
    let file_count = content.matches("### FILE:").count();
    let action_count = content.matches("### ACTION:").count();
    if file_count > 0 && action_count < file_count {
        return ValidationResult::Invalid {
            response: content.to_string(),
            reason: format!(
                "Found {} file markers but only {} action markers. \
                 Each ### FILE: must be followed by ### ACTION: create|modify|delete.",
                file_count, action_count
            ),
        };
    }

    ValidationResult::Valid(content.to_string())
}

/// Builds the correction prompt from previous failed attempts.
pub fn build_correction_prompt(original_request: &str, attempts: &[(String, String)]) -> String {
    let mut prompt = format!("Original request:\n{}\n\n", original_request);

    if attempts.is_empty() {
        return prompt;
    }

    prompt.push_str("Previous attempts failed. Here is the history:\n\n");

    for (i, (response, error)) in attempts.iter().enumerate() {
        prompt.push_str(&format!("--- Attempt {} ---\n", i + 1));
        prompt.push_str(&format!(
            "Response:\n{}\n\n",
            truncate_for_context(response, 2000)
        ));
        prompt.push_str(&format!("Error: {}\n\n", error));
    }

    prompt.push_str(
        "Please fix the issues above and provide a corrected response. \
                     Follow the required output format exactly.\n",
    );

    prompt
}

fn truncate_for_context(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        text.to_string()
    } else {
        let truncated = &text[..max_chars];
        format!(
            "{}...\n[truncated, {} chars omitted]",
            truncated,
            text.len() - max_chars
        )
    }
}

/// Runs a chat request with retry logic.
///
/// On validation failure, retries up to `max_retries` times, appending
/// previous failed responses and errors as context so the model can self-correct.
pub async fn chat_with_retry(
    client: &OllamaClient,
    model: &str,
    system_prompt: &str,
    user_request: &str,
    kind: ResponseKind,
    max_retries: usize,
    think: bool,
) -> RetryResult {
    let mut attempts: Vec<(String, String)> = Vec::new();
    let mut last_response = String::new();

    for attempt in 0..=max_retries {
        let messages = if attempt == 0 {
            vec![
                ChatMessage::system(system_prompt),
                ChatMessage::user(user_request),
            ]
        } else {
            let correction = build_correction_prompt(user_request, &attempts);
            vec![
                ChatMessage::system(system_prompt),
                ChatMessage::user(&correction),
            ]
        };

        let response = match client.chat(model, messages, think).await {
            Ok(r) => r,
            Err(e) => {
                return RetryResult::Failed {
                    last_error: format!("Ollama error: {:#}", e),
                    attempts: attempt,
                };
            }
        };

        last_response = response.content.clone();

        match validate_response(&response.content, kind) {
            ValidationResult::Valid(content) => {
                return RetryResult::Success {
                    content,
                    attempts: attempt,
                };
            }
            ValidationResult::Invalid { response, reason } => {
                attempts.push((response, reason));
            }
        }
    }

    // All retries exhausted — return last response anyway with a warning
    RetryResult::Exhausted {
        content: last_response,
        attempts: max_retries,
        corrections: attempts,
    }
}

/// Result of a retry-enabled chat request.
pub enum RetryResult {
    Success {
        content: String,
        attempts: usize,
    },
    Exhausted {
        content: String,
        attempts: usize,
        corrections: Vec<(String, String)>,
    },
    Failed {
        last_error: String,
        #[allow(dead_code)]
        attempts: usize,
    },
}

/// Unified result from the main event loop channel.
/// Wraps either a direct chat retry result or a full auto pipeline result.
pub enum PipelineResult {
    /// Direct chat or skill invocation result (non-streaming fallback)
    Retry(RetryResult),
    /// Auto pipeline completed — files were generated and applied
    AutoSuccess {
        changes: Vec<crate::agent::FileChange>,
        applied: Vec<String>,
    },
    /// Auto pipeline failed at some stage
    AutoFailed { error: String },
    /// Web search completed — results will be prepended to LLM context
    SearchDone {
        count: usize,
        #[allow(dead_code)]
        context: String,
    },
    /// Streaming chunk — token-by-token output from the LLM
    StreamChunk { content: String },
    /// Streaming finished — contains the full accumulated content
    StreamDone { content: String },
    /// Plan step completed — awaiting user approval before execution
    PlanReady { plan: String },
    /// A plan step is starting (for multi-step execution)
    StepStart { step: usize, total: usize, description: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_empty_response() {
        let result = validate_response("", ResponseKind::Chat);
        assert!(matches!(result, ValidationResult::Invalid { .. }));
    }

    #[test]
    fn validate_whitespace_response() {
        let result = validate_response("   \n  ", ResponseKind::Chat);
        assert!(matches!(result, ValidationResult::Invalid { .. }));
    }

    #[test]
    fn validate_valid_chat() {
        let result = validate_response("Here is some helpful text.", ResponseKind::Chat);
        assert!(matches!(result, ValidationResult::Valid(_)));
    }

    #[test]
    fn validate_code_missing_file_marker() {
        let result = validate_response(
            "I think you should create a file with hello world.",
            ResponseKind::CodeImplementation,
        );
        assert!(matches!(result, ValidationResult::Invalid { .. }));
        if let ValidationResult::Invalid { reason, .. } = result {
            assert!(reason.contains("### FILE:"));
        }
    }

    #[test]
    fn validate_code_missing_code_blocks() {
        let result = validate_response(
            "### FILE: main.rs\n### ACTION: create\nNo code block here.",
            ResponseKind::CodeImplementation,
        );
        assert!(matches!(result, ValidationResult::Invalid { .. }));
        if let ValidationResult::Invalid { reason, .. } = result {
            assert!(reason.contains("code blocks"));
        }
    }

    #[test]
    fn validate_code_unclosed_fence() {
        let result = validate_response(
            "### FILE: main.rs\n### ACTION: create\n```\nfn main() {}",
            ResponseKind::CodeImplementation,
        );
        assert!(matches!(result, ValidationResult::Invalid { .. }));
        if let ValidationResult::Invalid { reason, .. } = result {
            assert!(reason.contains("Unclosed"));
        }
    }

    #[test]
    fn validate_code_missing_action() {
        let result = validate_response(
            "### FILE: main.rs\n```\nfn main() {}\n```",
            ResponseKind::CodeImplementation,
        );
        assert!(matches!(result, ValidationResult::Invalid { .. }));
        if let ValidationResult::Invalid { reason, .. } = result {
            assert!(reason.contains("### ACTION:"));
        }
    }

    #[test]
    fn validate_code_valid() {
        let result = validate_response(
            "### FILE: main.rs\n### ACTION: create\n```\nfn main() {}\n```",
            ResponseKind::CodeImplementation,
        );
        assert!(matches!(result, ValidationResult::Valid(_)));
    }

    #[test]
    fn build_correction_includes_history() {
        let prompt = build_correction_prompt(
            "Create a hello world",
            &[
                ("bad response".into(), "Missing file markers".into()),
                ("still bad".into(), "Still wrong format".into()),
            ],
        );
        assert!(prompt.contains("Original request"));
        assert!(prompt.contains("Attempt 1"));
        assert!(prompt.contains("Attempt 2"));
        assert!(prompt.contains("bad response"));
        assert!(prompt.contains("Missing file markers"));
    }

    #[test]
    fn truncate_preserves_short_text() {
        let text = "short";
        assert_eq!(truncate_for_context(text, 100), text);
    }

    #[test]
    fn truncate_cuts_long_text() {
        let text = "a".repeat(3000);
        let truncated = truncate_for_context(&text, 2000);
        assert!(truncated.len() < text.len());
        assert!(truncated.contains("truncated"));
    }
}
