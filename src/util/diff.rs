use similar::{ChangeTag, TextDiff};

#[derive(Debug, Clone, PartialEq)]
pub enum DiffLine {
    Context(String),
    Added(String),
    Removed(String),
}

pub fn generate_diff(old: &str, new: &str) -> Vec<DiffLine> {
    let diff = TextDiff::from_lines(old, new);
    let mut result = Vec::new();
    for change in diff.iter_all_changes() {
        let line = change.to_string_lossy().into_owned();
        let dl = match change.tag() {
            ChangeTag::Equal => DiffLine::Context(line),
            ChangeTag::Delete => DiffLine::Removed(line),
            ChangeTag::Insert => DiffLine::Added(line),
        };
        result.push(dl);
    }
    result
}

pub fn generate_unified_diff(old: &str, new: &str, file_path: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut output = format!("--- {}\n+++ {}\n", file_path, file_path);
    for hunk in diff.unified_diff().header("", "").iter_hunks() {
        output.push_str(&hunk.to_string());
    }
    output
}

pub fn apply_diff(original: &str, added_lines: &[String], removed_lines: &[String]) -> String {
    let mut lines: Vec<String> = original.lines().map(|l| l.to_string()).collect();
    for removed in removed_lines {
        lines.retain(|l| l.trim_end() != removed.trim_end());
    }
    for added in added_lines {
        lines.push(added.clone());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_additions() {
        let old = "line1\nline2\n";
        let new = "line1\nline2\nline3\n";
        let diff = generate_diff(old, new);
        let added: Vec<_> = diff.iter().filter(|d| matches!(d, DiffLine::Added(_))).collect();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0], &DiffLine::Added("line3\n".into()));
    }

    #[test]
    fn diff_removals() {
        let old = "a\nb\nc\n";
        let new = "a\nc\n";
        let diff = generate_diff(old, new);
        let removed: Vec<_> = diff.iter().filter(|d| matches!(d, DiffLine::Removed(_))).collect();
        assert_eq!(removed.len(), 1);
    }

    #[test]
    fn diff_identical_is_empty_changes() {
        let content = "hello\nworld\n";
        let diff = generate_diff(content, content);
        assert!(diff.iter().all(|d| matches!(d, DiffLine::Context(_))));
    }

    #[test]
    fn unified_diff_format() {
        let old = "foo\nbar\n";
        let new = "foo\nbaz\n";
        let result = generate_unified_diff(old, new, "test.txt");
        assert!(result.starts_with("--- test.txt"));
        assert!(result.contains("+++ test.txt"));
        assert!(result.contains("-bar"));
        assert!(result.contains("+baz"));
    }
}
