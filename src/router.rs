// Heuristic request router: selects model tier based on request content.
// Routes: simple Q&A -> fast_model, code -> core_model, reviews -> audit_model.

/// Which model tier to route a request to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    Fast,
    Core,
    Audit,
}

/// Classify a user request into a model tier based on keyword heuristics.
pub fn classify_request(input: &str) -> ModelTier {
    let lower = input.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    // Audit patterns — review, audit, check, verify, test
    let audit_patterns = [
        "review",
        "audit",
        "check",
        "verify",
        "test",
        "inspect",
        "analyze",
        "security",
        "vulnerability",
    ];
    if contains_any(&words, &audit_patterns) {
        return ModelTier::Audit;
    }

    // Core patterns — create, fix, implement, refactor, build
    let core_patterns = [
        "create",
        "make",
        "build",
        "implement",
        "fix",
        "bug",
        "error",
        "refactor",
        "rewrite",
        "modify",
        "change",
        "update",
        "add",
        "remove",
        "delete",
        "write",
        "generate",
        "### file",
        "apply",
    ];
    if contains_any(&words, &core_patterns) {
        return ModelTier::Core;
    }

    // Fast patterns — questions, explanations, simple lookups
    let fast_patterns = [
        "what",
        "why",
        "how",
        "explain",
        "describe",
        "tell me",
        "show me",
        "list",
        "help",
        "is it",
        "can you",
        "does",
        "difference",
        "mean",
    ];
    if contains_any(&words, &fast_patterns) {
        return ModelTier::Fast;
    }

    // Default: short inputs likely simple, long inputs likely complex
    if words.len() <= 10 {
        ModelTier::Fast
    } else {
        ModelTier::Core
    }
}

/// Check if any pattern appears as a whole word or as a substring in any word.
fn contains_any(words: &[&str], patterns: &[&str]) -> bool {
    let flat = words.join(" ");
    patterns.iter().any(|p| flat.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_question_routes_fast() {
        assert_eq!(classify_request("What is Rust?"), ModelTier::Fast);
        assert_eq!(
            classify_request("How does borrowing work?"),
            ModelTier::Fast
        );
        assert_eq!(classify_request("Explain this function"), ModelTier::Fast);
    }

    #[test]
    fn code_creation_routes_core() {
        assert_eq!(
            classify_request("Create a REST API with authentication"),
            ModelTier::Core
        );
        assert_eq!(
            classify_request("Fix the bug in login flow"),
            ModelTier::Core
        );
        assert_eq!(
            classify_request("Add error handling to the parser"),
            ModelTier::Core
        );
    }

    #[test]
    fn review_routes_audit() {
        assert_eq!(
            classify_request("Review this code for security issues"),
            ModelTier::Audit
        );
        assert_eq!(
            classify_request("Check if there are any bugs"),
            ModelTier::Audit
        );
        assert_eq!(
            classify_request("Analyze the performance of this module"),
            ModelTier::Audit
        );
    }

    #[test]
    fn short_input_defaults_fast() {
        assert_eq!(classify_request("hello"), ModelTier::Fast);
        assert_eq!(classify_request("list files"), ModelTier::Fast);
    }

    #[test]
    fn long_input_defaults_core() {
        let long = "I have a complex system that needs to handle concurrent requests with proper error handling and retry logic across multiple services";
        assert_eq!(classify_request(long), ModelTier::Core);
    }

    #[test]
    fn audit_beats_core_patterns() {
        // "test" is an audit pattern, should win over potential core patterns
        assert_eq!(
            classify_request("Write test cases for the parser"),
            ModelTier::Audit
        );
    }
}
