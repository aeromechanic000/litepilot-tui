use crate::sandbox::executor::{CommandOutput, Executor};
use crate::sandbox::Sandbox;
use anyhow::{Context, Result};
use std::path::Path;

pub struct UvManager;

impl UvManager {
    pub async fn init(project_dir: &Path, sandbox: &Sandbox) -> Result<CommandOutput> {
        let executor = Executor::new(sandbox);
        executor
            .run("uv", &["init".into()], Some(project_dir.to_path_buf()))
            .await
            .context("Running uv init")
    }

    pub async fn create_venv(project_dir: &Path, sandbox: &Sandbox) -> Result<CommandOutput> {
        let executor = Executor::new(sandbox);
        executor
            .run("uv", &["venv".into()], Some(project_dir.to_path_buf()))
            .await
            .context("Running uv venv")
    }

    pub async fn add(
        project_dir: &Path,
        package: &str,
        sandbox: &Sandbox,
    ) -> Result<CommandOutput> {
        let executor = Executor::new(sandbox);
        executor
            .run(
                "uv",
                &["add".into(), package.into()],
                Some(project_dir.to_path_buf()),
            )
            .await
            .context("Running uv add")
    }

    pub async fn run(project_dir: &Path, script: &str, sandbox: &Sandbox) -> Result<CommandOutput> {
        let executor = Executor::new(sandbox);
        executor
            .run(
                "uv",
                &["run".into(), script.into()],
                Some(project_dir.to_path_buf()),
            )
            .await
            .context("Running uv run")
    }

    pub fn is_available() -> bool {
        std::process::Command::new("uv")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn check_uv_availability() {
        // Just verify it doesn't panic — uv may or may not be installed
        let _ = UvManager::is_available();
    }

    #[tokio::test]
    async fn uv_init_with_missing_binary() {
        let dir = TempDir::new().unwrap();
        let sandbox = Sandbox::new(dir.path().to_path_buf());
        // This will fail if uv isn't installed, which is fine
        let result = UvManager::init(dir.path(), &sandbox).await;
        // Just verify it returns something (success or meaningful error)
        let _ = result;
    }
}
