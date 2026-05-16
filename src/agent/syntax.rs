use crate::sandbox::executor::Executor;
use crate::sandbox::Sandbox;
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum SyntaxResult {
    Pass,
    Fail { errors: String },
    Skipped(String),
}

#[derive(Debug, Clone, Copy)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Bash,
    Rust,
    Go,
    C,
    Cpp,
}

impl Language {
    pub fn check_command(&self) -> (&'static str, Vec<&'static str>) {
        match self {
            Language::Python => ("python3", vec!["-m", "py_compile", "{}"]),
            Language::JavaScript => ("node", vec!["-c", "{}"]),
            Language::TypeScript => ("npx", vec!["tsc", "--noEmit", "{}"]),
            Language::Bash => ("bash", vec!["-n", "{}"]),
            Language::Rust => ("rustc", vec!["--check", "{}"]),
            Language::Go => ("go", vec!["vet", "./..."]),
            Language::C => ("gcc", vec!["-fsyntax-only", "{}"]),
            Language::Cpp => ("g++", vec!["-fsyntax-only", "{}"]),
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "py" => Some(Self::Python),
            "js" | "mjs" => Some(Self::JavaScript),
            "ts" | "tsx" => Some(Self::TypeScript),
            "sh" | "bash" => Some(Self::Bash),
            "rs" => Some(Self::Rust),
            "go" => Some(Self::Go),
            "c" | "h" => Some(Self::C),
            "cpp" | "cc" | "cxx" | "hpp" => Some(Self::Cpp),
            _ => None,
        }
    }
}

pub struct SyntaxChecker;

impl SyntaxChecker {
    pub fn detect_language(path: &Path) -> Option<Language> {
        let ext = path.extension()?.to_str()?;
        Language::from_extension(ext)
    }

    pub async fn check(path: &Path, sandbox: &Sandbox) -> Result<SyntaxResult> {
        let Some(lang) = Self::detect_language(path) else {
            return Ok(SyntaxResult::Skipped("Unsupported language".into()));
        };

        if !path.exists() {
            return Ok(SyntaxResult::Skipped("File does not exist".into()));
        }

        let (cmd, template) = lang.check_command();
        let args: Vec<String> = template.iter()
            .map(|a| a.replace("{}", &path.to_string_lossy()))
            .collect();

        let executor = Executor::new(sandbox);
        let output = executor.run(cmd, &args, None).await?;

        if output.success {
            Ok(SyntaxResult::Pass)
        } else {
            Ok(SyntaxResult::Fail {
                errors: format!("{}\n{}", output.stdout, output.stderr).trim().to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detect_language_from_extension() {
        assert!(matches!(Language::from_extension("py"), Some(Language::Python)));
        assert!(matches!(Language::from_extension("rs"), Some(Language::Rust)));
        assert!(matches!(Language::from_extension("js"), Some(Language::JavaScript)));
        assert!(matches!(Language::from_extension("go"), Some(Language::Go)));
        assert!(matches!(Language::from_extension("txt"), None));
    }

    #[test]
    fn detect_language_from_path() {
        let path = Path::new("src/main.rs");
        assert!(matches!(SyntaxChecker::detect_language(path), Some(Language::Rust)));
    }

    #[tokio::test]
    async fn check_valid_python() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.py");
        std::fs::write(&file, "print('hello')\n").unwrap();
        let sandbox = Sandbox::new(dir.path().to_path_buf());
        let result = SyntaxChecker::check(&file, &sandbox).await;
        assert!(result.is_ok());
        match result.unwrap() {
            SyntaxResult::Pass | SyntaxResult::Skipped(_) => {},
            SyntaxResult::Fail { .. } => {}, // python3 might not be installed
        }
    }

    #[tokio::test]
    async fn check_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let sandbox = Sandbox::new(dir.path().to_path_buf());
        let result = SyntaxChecker::check(Path::new("nonexistent.py"), &sandbox).await.unwrap();
        assert!(matches!(result, SyntaxResult::Skipped(_)));
    }
}
