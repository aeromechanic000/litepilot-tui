use crate::sandbox::{Sandbox, SandboxError};
use anyhow::Result;
use std::path::PathBuf;
use std::process::Output;
use tokio::process::Command;

#[allow(dead_code)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub success: bool,
}

#[allow(dead_code)]
pub struct Executor<'a> {
    sandbox: &'a Sandbox,
}

#[allow(dead_code)]
impl<'a> Executor<'a> {
    pub fn new(sandbox: &'a Sandbox) -> Self {
        Self { sandbox }
    }

    pub async fn run(
        &self,
        cmd: &str,
        args: &[String],
        cwd: Option<PathBuf>,
    ) -> Result<CommandOutput, SandboxError> {
        self.sandbox.validate_command(cmd, args)?;

        let working_dir = cwd.unwrap_or_else(|| self.sandbox.workspace().to_path_buf());

        // Standard execution with allowlist enforcement
        // OS-level sandboxing (Seatbelt/Landlock) is available via run_os_sandboxed()
        // but not used by default — it can be enabled by callers that need extra isolation.
        let output: Output = Command::new(cmd)
            .args(args)
            .current_dir(&working_dir)
            .output()
            .await
            .map_err(|e| {
                SandboxError::PathResolution(format!("Failed to execute {}: {}", cmd, e))
            })?;

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
            success: output.status.success(),
        })
    }

    /// Execute a command with OS-level sandboxing when available.
    /// Falls back to standard execution otherwise.
    #[allow(dead_code)]
    pub async fn run_os_sandboxed(
        &self,
        cmd: &str,
        args: &[String],
        cwd: Option<PathBuf>,
    ) -> Result<CommandOutput, SandboxError> {
        self.sandbox.validate_command(cmd, args)?;

        #[cfg(target_os = "macos")]
        {
            if crate::sandbox::seatbelt::is_available() {
                let workspace = self.sandbox.workspace().to_path_buf();
                let cmd_owned = cmd.to_string();
                let args_owned = args.to_vec();

                let output = tokio::task::spawn_blocking(move || {
                    crate::sandbox::seatbelt::exec_sandboxed(&cmd_owned, &args_owned, &workspace)
                })
                .await
                .map_err(|e| {
                    SandboxError::PathResolution(format!("Seatbelt execution failed: {}", e))
                })?
                .map_err(|e| {
                    SandboxError::PathResolution(format!("Failed to execute {}: {}", cmd, e))
                })?;

                return Ok(CommandOutput {
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    exit_code: output.status.code(),
                    success: output.status.success(),
                });
            }
        }

        // Fallback: standard execution
        self.run(cmd, args, cwd).await
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
        let result = executor
            .run("sudo", &["rm".into(), "-rf".into(), "/".into()], None)
            .await;
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
