use crate::app::ConversationMessage;
use crate::ollama::chat::ChatMessage;
use crate::ollama::model::estimate_context_window;
use crate::util::text::estimate_tokens;

/// Build the message array for an LLM request, fitting conversation history
/// within the model's context window. Reserves 10% for the current request.
pub fn build_messages(
    system_prompt: &str,
    history: &[ConversationMessage],
    user_request: &str,
    plan: Option<&str>,
    core_model: &str,
) -> Vec<ChatMessage> {
    let context_window = estimate_context_window(core_model) as usize;
    let reserve = (context_window as f64 * 0.10) as usize;

    let mut messages = vec![ChatMessage::system(system_prompt)];
    let mut used = estimate_tokens(system_prompt);

    // Add plan as system context if present
    if let Some(plan_text) = plan {
        let plan_str = format!("[Plan]\n{}", plan_text);
        let plan_tokens = estimate_tokens(&plan_str);
        messages.push(ChatMessage::system(&plan_str));
        used += plan_tokens;
    }

    // Walk history newest→oldest, prepend within budget
    let available = context_window.saturating_sub(reserve).saturating_sub(used);
    let mut history_used = 0;
    let mut history_messages = Vec::new();
    for msg in history.iter().rev() {
        if history_used + msg.tokens > available {
            break;
        }
        history_messages.push(ChatMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
        });
        history_used += msg.tokens;
    }
    history_messages.reverse();

    // Insert history after system prompts, before user request
    messages.extend(history_messages);

    // Always add the current user request
    messages.push(ChatMessage::user(user_request));

    messages
}

/// Sum of all message tokens in the history.
pub fn estimate_history_tokens(history: &[ConversationMessage]) -> usize {
    history.iter().map(|m| m.tokens).sum()
}

/// Check if history exceeds 90% of the context window and compact if needed.
/// Simple truncation: keeps the most recent messages that fit within 60% of
/// the window, discarding older ones.
pub fn maybe_compact(history: &mut Vec<ConversationMessage>, core_model: &str) {
    let context_window = estimate_context_window(core_model) as usize;
    let threshold = (context_window as f64 * 0.90) as usize;
    let total = estimate_history_tokens(history);

    if total <= threshold || history.is_empty() {
        return;
    }

    // Keep recent messages that fit within 60% of context window
    let keep_budget = (context_window as f64 * 0.60) as usize;
    let mut kept_tokens = 0;
    let split_point = history
        .iter()
        .rev()
        .position(|m| {
            if kept_tokens + m.tokens > keep_budget {
                return true;
            }
            kept_tokens += m.tokens;
            false
        })
        .map(|p| history.len().saturating_sub(p))
        .unwrap_or(0);

    if split_point > 0 {
        history.drain(0..split_point);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: &str, content: &str) -> ConversationMessage {
        ConversationMessage {
            role: role.to_string(),
            content: content.to_string(),
            tokens: estimate_tokens(content),
        }
    }

    #[test]
    fn build_messages_includes_system_and_user() {
        let msgs = build_messages("system", &[], "hello", None, "qwen3:8b");
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "system");
        assert_eq!(msgs[1].role, "user");
    }

    #[test]
    fn build_messages_includes_plan() {
        let msgs = build_messages("system", &[], "hello", Some("step 1"), "qwen3:8b");
        assert_eq!(msgs.len(), 3);
        assert!(msgs[1].content.contains("[Plan]"));
    }

    #[test]
    fn build_messages_includes_history() {
        let history = vec![
            msg("user", "first question"),
            msg("assistant", "first answer"),
        ];
        let msgs = build_messages("system", &history, "second question", None, "qwen3:8b");
        // system + 2 history + user = 4
        assert_eq!(msgs.len(), 4);
        assert_eq!(msgs[1].role, "user");
        assert_eq!(msgs[1].content, "first question");
    }

    #[test]
    fn build_messages_truncates_old_history() {
        // qwen3:1b → 2048 context. Fill with ~3000 tokens worth of messages.
        let mut history = Vec::new();
        for i in 0..120 {
            // Each message ~14 tokens (56 chars), so 120*14 = 1680 tokens total
            history.push(msg("user", &format!("message number {:03} with enough padding text to fill up", i)));
            history.push(msg("assistant", &format!("response {:03} with sufficient text padding content here now", i)));
        }
        // Total ~240 messages * ~14 tokens = ~3360 tokens > 2048 * 0.9
        let msgs = build_messages("system", &history, "latest", None, "qwen3:1b");
        // Should not include all history
        assert!(msgs.len() < history.len() + 2, "got {} msgs vs {} history+2", msgs.len(), history.len() + 2);
        assert_eq!(msgs.last().unwrap().content, "latest");
    }

    #[test]
    fn maybe_compact_does_nothing_when_under_threshold() {
        let mut history = vec![msg("user", "hello")];
        maybe_compact(&mut history, "qwen3:8b");
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn maybe_compact_truncates_when_over_threshold() {
        // qwen3:2b → 4096 context. Fill with ~6000 tokens to exceed 90% threshold (3686).
        let mut history = Vec::new();
        for i in 0..400 {
            history.push(msg("user", &format!("message number {:03} with enough text to use tokens and fill context", i)));
        }
        // Each ~16 tokens, 400 * 16 = 6400 tokens > 3686
        let before = history.len();
        maybe_compact(&mut history, "qwen3:2b");
        assert!(history.len() < before, "history was not compacted: {} vs {}", history.len(), before);
        assert!(history.last().unwrap().content.contains("399"));
    }

    #[test]
    fn estimate_history_tokens_works() {
        let history = vec![
            msg("user", "hello world"),     // ~3 tokens
            msg("assistant", "hi there"),   // ~2 tokens
        ];
        let total = estimate_history_tokens(&history);
        assert!(total > 0);
    }
}
