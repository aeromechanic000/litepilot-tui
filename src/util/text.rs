#[allow(dead_code)]
pub fn estimate_tokens(text: &str) -> usize {
    // Rough estimate: ~4 chars per token for English/code
    text.chars().count().div_ceil(4)
}

#[allow(dead_code)]
pub fn truncate_to_tokens(text: &str, max_tokens: usize) -> &str {
    let max_chars = max_tokens * 4;
    if text.len() <= max_chars {
        return text;
    }
    // Find a safe char boundary
    let mut end = max_chars;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[allow(dead_code)]
pub fn truncate_lines(text: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= max_lines {
        return text.to_string();
    }
    let kept: Vec<&str> = lines.into_iter().take(max_lines).collect();
    format!("{}\n... (truncated {} lines)", kept.join("\n"), 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_estimation() {
        assert!(estimate_tokens("hello world") > 0);
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    #[test]
    fn truncate_preserves_short_text() {
        assert_eq!(truncate_to_tokens("short", 100), "short");
    }

    #[test]
    fn truncate_cuts_long_text() {
        let long = "a".repeat(1000);
        let result = truncate_to_tokens(&long, 10);
        assert!(result.len() <= 40);
    }

    #[test]
    fn truncate_lines_keeps_count() {
        let text = "line1\nline2\nline3\nline4\nline5";
        let result = truncate_lines(text, 3);
        assert!(result.starts_with("line1"));
        assert!(result.contains("truncated"));
    }
}
