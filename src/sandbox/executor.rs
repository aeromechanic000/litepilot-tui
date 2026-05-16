use crate::sandbox::{Sandbox, SandboxError};
use anyhow::Result;
use std::path::PathBuf;
use std::process::Output;
use tokio::process::Command;

pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub success: bool,
}

pub struct Executor<'a> {
    sandbox: &'a Sandbox,
}

impl<'a> Executor<'a> {
    pub fn new(sandbox: &'a Sandbox) -> Self {
        Self { sandbox }
    }

    pub async fn run(&self, cmd: &str, args: &[String], cwd: Option<PathBuf>) -> Result<CommandOutput, SandboxError> {
        self.sandbox.validate_command(cmd, args)?;

        let working_dir = cwd.unwrap_or_else(|| self.sandbox.workspace().to_path_buf());

        let output: Output = Command::new(cmd)
            .args(args)
            .current_dir(&working_dir)
            .output()
            .await
            .map_err(|e| SandboxError::PathResolution(format!("Failed to execute {}: {}", cmd, e)))?;

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
            success: output.status.success(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn execute_echo() {
        let dir = TempDir::new().unwrap();
        let sandbox = Sandbox::new(dir.path().to_path_buf());
        let executor = Executor::new(&sandbox);
        let result = executor.run("echo", &["hello".into()], None).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);
        assert!(output.stdout.trim() == "hello");
    }

    #[tokio::test]
    async fn execute_blocked_command() {
        let dir = TempDir::new().unwrap();
        let sandbox = Sandbox::new(dir.path().to_path_buf());
        let executor = Executor::new(&sandbox);
        let result = executor.run("sudo", &["rm".into(), "-rf".into(), "/".into()], None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_in_cwd() {
        let dir = TempDir::new().unwrap();
        let sandbox = Sandbox::new(dir.path().to_path_buf());
        let executor = Executor::new(&sandbox);
        let result = executor.run("pwd", &[], None).await;
        assert!(result.is_ok());
        let output_path = result.unwrap().stdout.trim().to_string();
        let expected = dir.path().canonicalize().unwrap();
        let actual = std::path::Path::new(&output_path).canonicalize().unwrap();
        assert_eq!(actual, expected);
    }
}
