use crate::agent::syntax::SyntaxChecker;
use crate::sandbox::Sandbox;
use std::path::Path;

/// A single diagnostic error from post-write checking.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DiagnosticError {
    pub file: String,
    pub line: Option<usize>,
    pub message: String,
}

/// Result of running diagnostics on one or more files.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DiagnosticResult {
    pub errors: Vec<DiagnosticError>,
    pub files_checked: usize,
    pub files_skipped: usize,
}

impl DiagnosticResult {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Format errors for injection into a correction prompt.
    #[allow(dead_code)]
    pub fn format_for_correction(&self) -> String {
        if self.errors.is_empty() {
            return String::new();
        }

        let mut parts = Vec::with_capacity(self.errors.len());
        for err in &self.errors {
            let location = match err.line {
                Some(l) => format!("{}:{}", err.file, l),
                None => err.file.clone(),
            };
            parts.push(format!("- {}: {}", location, err.message));
        }

        format!(
            "The following errors were detected after writing files:\n{}\n\n\
             Fix these errors. Output the corrected file(s) using the standard format.",
            parts.join("\n")
        )
    }
}

/// Run syntax diagnostics on the given file paths.
/// Non-blocking: individual file check failures are recorded as errors,
/// but the overall function never fails.
#[allow(dead_code)]
pub async fn run_diagnostics(paths: &[impl AsRef<Path>], sandbox: &Sandbox) -> DiagnosticResult {
    let mut errors = Vec::new();
    let mut files_checked = 0;
    let mut files_skipped = 0;

    for path in paths {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_string();

        // Skip files that don't exist yet (shouldn't happen post-write, but be safe)
        if !path.exists() {
            files_skipped += 1;
            continue;
        }

        match SyntaxChecker::check(path, sandbox).await {
            Ok(result) => match result {
                crate::agent::syntax::SyntaxResult::Pass => {
                    files_checked += 1;
                }
                crate::agent::syntax::SyntaxResult::Fail { errors: err_text } => {
                    files_checked += 1;
                    for line in err_text.lines().take(10) {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        let (file, line_num, message) = parse_error_line(trimmed, &path_str);
                        errors.push(DiagnosticError {
                            file,
                            line: line_num,
                            message,
                        });
                    }
                }
                crate::agent::syntax::SyntaxResult::Skipped(_) => {
                    files_skipped += 1;
                }
            },
            Err(_) => {
                files_skipped += 1;
            }
        }
    }

    DiagnosticResult {
        errors,
        files_checked,
        files_skipped,
    }
}

/// Attempt to parse a compiler/linter error line into (file, line_number, message).
/// Handles common formats like:
/// - `file.py:5: SyntaxError: invalid syntax`
/// - `error[E0425]: cannot find value` (Rust)
/// - `file.rs:10:5: error[E0425]`
fn parse_error_line(line: &str, fallback_file: &str) -> (String, Option<usize>, String) {
    // Try file:line:col: message pattern
    let parts: Vec<&str> = line.splitn(4, ':').collect();
    if parts.len() >= 3 {
        if let Ok(line_num) = parts[1].trim().parse::<usize>() {
            let file = parts[0].trim().to_string();
            let message = parts[2..].join(":").trim().to_string();
            return (file, Some(line_num), message);
        }
    }

    // Fallback: no structured parse, use the whole line
    (fallback_file.to_string(), None, line.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_result_has_errors() {
        let result = DiagnosticResult {
            errors: vec![DiagnosticError {
                file: "test.py".into(),
                line: Some(5),
                message: "SyntaxError".into(),
            }],
            files_checked: 1,
            files_skipped: 0,
        };
        assert!(result.has_errors());
    }

    #[test]
    fn diagnostic_result_no_errors() {
        let result = DiagnosticResult {
            errors: vec![],
            files_checked: 1,
            files_skipped: 0,
        };
        assert!(!result.has_errors());
    }

    #[test]
    fn format_for_correction_includes_errors() {
        let result = DiagnosticResult {
            errors: vec![
                DiagnosticError {
                    file: "main.rs".into(),
                    line: Some(10),
                    message: "cannot find value `x`".into(),
                },
                DiagnosticError {
                    file: "lib.rs".into(),
                    line: None,
                    message: "mismatched types".into(),
                },
            ],
            files_checked: 2,
            files_skipped: 0,
        };
        let formatted = result.format_for_correction();
        assert!(formatted.contains("main.rs:10"));
        assert!(formatted.contains("cannot find value"));
        assert!(formatted.contains("lib.rs"));
        assert!(formatted.contains("mismatched types"));
        assert!(formatted.contains("Fix these errors"));
    }

    #[test]
    fn format_empty_errors() {
        let result = DiagnosticResult {
            errors: vec![],
            files_checked: 1,
            files_skipped: 0,
        };
        assert!(result.format_for_correction().is_empty());
    }

    #[test]
    fn parse_error_line_with_location() {
        let (file, line, msg) =
            parse_error_line("test.py:5:10: SyntaxError: invalid syntax", "fallback.py");
        assert_eq!(file, "test.py");
        assert_eq!(line, Some(5));
        assert!(msg.contains("SyntaxError"));
    }

    #[test]
    fn parse_error_line_no_colon_structure() {
        let (file, line, msg) = parse_error_line("some random error text", "fallback.rs");
        assert_eq!(file, "fallback.rs");
        assert_eq!(line, None);
        assert_eq!(msg, "some random error text");
    }

    #[test]
    fn parse_error_line_non_numeric_after_first_colon() {
        let (file, line, msg) = parse_error_line("error: something went wrong", "test.rs");
        // First colon splits as ["error", " something went wrong"] — not enough parts
        // Actually with splitn(4, ':') we get ["error", " something went wrong"] (2 parts)
        // parts[1].trim().parse::<usize>() fails, so fallback
        assert_eq!(line, None);
    }
}
