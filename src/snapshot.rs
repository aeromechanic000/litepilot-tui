use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Manages side-git snapshots for undo/rollback without touching the user's
/// own git repository. Uses `git --git-dir` / `--work-tree` to maintain a
/// separate object store at `~/.litepilot/snapshots/<hash>/`.
pub struct SnapshotManager {
    git_dir: PathBuf,
    work_tree: PathBuf,
}

/// Maximum snapshots to retain.
#[allow(dead_code)]
const MAX_SNAPSHOTS: usize = 50;
/// Maximum age in days before pruning.
#[allow(dead_code)]
const MAX_AGE_DAYS: u64 = 7;

impl SnapshotManager {
    /// Create a SnapshotManager for the given workspace and config directory.
    pub fn new(workspace: &Path, config_dir: &Path) -> Self {
        let hash = workspace_hash(workspace);
        let git_dir = config_dir.join("snapshots").join(hash);
        Self {
            git_dir,
            work_tree: workspace.to_path_buf(),
        }
    }

    /// Initialize the side-git repo if it doesn't exist.
    pub fn init(&self) -> Result<()> {
        if self.git_dir.join("HEAD").exists() {
            return Ok(());
        }
        std::fs::create_dir_all(&self.git_dir)
            .with_context(|| format!("Creating snapshot dir {:?}", self.git_dir))?;

        self.git_cmd(&["init", "--bare"])?;
        // Set a default identity so commits don't fail
        self.git_cmd_config("user.name", "litepilot")?;
        self.git_cmd_config("user.email", "litepilot@local")?;
        Ok(())
    }

    /// Take a pre-turn snapshot. Returns commit hash on success.
    pub fn pre_turn(&self, label: &str) -> Result<String> {
        self.init()?;
        self.stage_all()?;
        let message = format!("pre-turn: {}", label);
        self.commit(&message)
    }

    /// Take a post-turn snapshot. Returns commit hash on success.
    pub fn post_turn(&self, label: &str) -> Result<String> {
        self.stage_all()?;
        let message = format!("post-turn: {}", label);
        self.commit(&message)
    }

    /// Restore the workspace to a specific snapshot commit.
    pub fn restore(&self, commit: &str) -> Result<()> {
        self.init()?;
        // Force checkout the commit, overwriting working tree
        self.git_cmd(&[
            "--work-tree",
            self.work_tree.to_str().unwrap_or("."),
            "checkout",
            "--force",
            commit,
            "--",
            ".",
        ])?;
        Ok(())
    }

    /// List recent snapshots (commit hash + message + date), newest first.
    pub fn list(&self, max: usize) -> Result<Vec<SnapshotEntry>> {
        if !self.git_dir.join("HEAD").exists() {
            return Ok(Vec::new());
        }
        let output =
            self.git_cmd_output(&["log", &format!("-{}", max), "--pretty=format:%H|%s|%ci"])?;

        let mut entries = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() == 3 {
                entries.push(SnapshotEntry {
                    hash: parts[0].to_string(),
                    message: parts[1].to_string(),
                    date: parts[2].to_string(),
                });
            }
        }
        Ok(entries)
    }

    /// Prune old snapshots beyond retention limits.
    #[allow(dead_code)]
    pub fn prune(&self) -> Result<()> {
        if !self.git_dir.join("HEAD").exists() {
            return Ok(());
        }

        // Prune by count
        let entries = self.list(MAX_SNAPSHOTS + 1)?;
        if entries.len() > MAX_SNAPSHOTS {
            // Keep only the newest MAX_SNAPSHOTS
            let cutoff = &entries[MAX_SNAPSHOTS - 1].hash;
            // Reset to cutoff, older commits become unreachable
            let _ = self.git_cmd(&["reset", "--soft", cutoff]);
        }

        // Prune by age — remove git objects older than MAX_AGE_DAYS
        let _ = self.git_cmd(&[
            "reflog",
            "expire",
            &format!("--expire={}.days.ago", MAX_AGE_DAYS),
            "--all",
        ]);
        let _ = self.git_cmd(&["gc", "--prune=now"]);

        Ok(())
    }

    fn stage_all(&self) -> Result<()> {
        // Add all files in workspace
        self.git_cmd(&[
            "--work-tree",
            self.work_tree.to_str().unwrap_or("."),
            "add",
            "-A",
        ])?;
        Ok(())
    }

    fn commit(&self, message: &str) -> Result<String> {
        // Check if there's anything to commit
        let status = self.git_cmd_output(&[
            "--work-tree",
            self.work_tree.to_str().unwrap_or("."),
            "status",
            "--porcelain",
        ])?;
        if status.trim().is_empty() {
            // Nothing to commit — return current HEAD
            return self
                .git_cmd_output(&["rev-parse", "HEAD"])
                .map(|s| s.trim().to_string())
                .map_err(|e| anyhow::anyhow!("No changes to snapshot: {}", e));
        }
        self.git_cmd(&[
            "--work-tree",
            self.work_tree.to_str().unwrap_or("."),
            "commit",
            "-m",
            message,
        ])?;
        // Get the actual commit hash
        self.git_cmd_output(&["rev-parse", "HEAD"])
            .map(|s| s.trim().to_string())
            .map_err(|e| anyhow::anyhow!("Snapshot commit failed: {}", e))
    }

    fn git_cmd(&self, args: &[&str]) -> Result<()> {
        let mut cmd = Command::new("git");
        cmd.arg(format!("--git-dir={}", self.git_dir.display()));
        for arg in args {
            cmd.arg(arg);
        }
        cmd.env("GIT_AUTHOR_NAME", "litepilot");
        cmd.env("GIT_AUTHOR_EMAIL", "litepilot@local");
        cmd.env("GIT_COMMITTER_NAME", "litepilot");
        cmd.env("GIT_COMMITTER_EMAIL", "litepilot@local");

        let status = cmd.status().with_context(|| "Failed to run git")?;
        if !status.success() {
            anyhow::bail!("git command failed: {:?} (exit {:?})", args, status.code());
        }
        Ok(())
    }

    fn git_cmd_config(&self, key: &str, value: &str) -> Result<()> {
        let status = Command::new("git")
            .arg(format!("--git-dir={}", self.git_dir.display()))
            .arg("config")
            .arg(key)
            .arg(value)
            .status()
            .with_context(|| "Failed to run git config")?;
        if !status.success() {
            anyhow::bail!("git config {} {} failed", key, value);
        }
        Ok(())
    }

    fn git_cmd_output(&self, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new("git");
        cmd.arg(format!("--git-dir={}", self.git_dir.display()));
        for arg in args {
            cmd.arg(arg);
        }
        cmd.env("GIT_AUTHOR_NAME", "litepilot");
        cmd.env("GIT_AUTHOR_EMAIL", "litepilot@local");
        cmd.env("GIT_COMMITTER_NAME", "litepilot");
        cmd.env("GIT_COMMITTER_EMAIL", "litepilot@local");

        let output = cmd.output().with_context(|| "Failed to run git")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git {:?}: {}", args, stderr.trim());
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[derive(Debug, Clone)]
pub struct SnapshotEntry {
    pub hash: String,
    pub message: String,
    pub date: String,
}

/// Generate a stable directory name from the workspace path.
fn workspace_hash(workspace: &Path) -> String {
    let path_str = workspace.to_string_lossy();
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path_str.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn workspace_hash_is_stable() {
        let p = Path::new("/tmp/test-project");
        let h1 = workspace_hash(p);
        let h2 = workspace_hash(p);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16);
    }

    #[test]
    fn workspace_hash_differs_for_different_paths() {
        let h1 = workspace_hash(Path::new("/tmp/project-a"));
        let h2 = workspace_hash(Path::new("/tmp/project-b"));
        assert_ne!(h1, h2);
    }

    #[test]
    fn snapshot_manager_creation() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join("project");
        std::fs::create_dir_all(&workspace).unwrap();
        let config_dir = tmp.path().join(".litepilot");
        let mgr = SnapshotManager::new(&workspace, &config_dir);
        assert!(mgr.git_dir.ends_with(&workspace_hash(&workspace)));
        assert_eq!(mgr.work_tree, workspace);
    }

    #[test]
    fn init_creates_bare_repo() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();
        let config_dir = tmp.path().join("lp");
        let mgr = SnapshotManager::new(&workspace, &config_dir);
        mgr.init().unwrap();
        assert!(mgr.git_dir.join("HEAD").exists());
    }

    #[test]
    fn init_idempotent() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();
        let config_dir = tmp.path().join("lp");
        let mgr = SnapshotManager::new(&workspace, &config_dir);
        mgr.init().unwrap();
        mgr.init().unwrap(); // Should not error
        assert!(mgr.git_dir.join("HEAD").exists());
    }

    #[test]
    fn list_empty_when_no_snapshots() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();
        let config_dir = tmp.path().join("lp");
        let mgr = SnapshotManager::new(&workspace, &config_dir);
        let entries = mgr.list(10).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn pre_post_turn_creates_snapshots() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();
        // Create a file so there's something to snapshot
        std::fs::write(workspace.join("hello.txt"), "hello").unwrap();
        let config_dir = tmp.path().join("lp");
        let mgr = SnapshotManager::new(&workspace, &config_dir);

        let hash1 = mgr.pre_turn("test task").unwrap();
        assert!(!hash1.is_empty());

        // Modify and post-turn
        std::fs::write(workspace.join("hello.txt"), "world").unwrap();
        let hash2 = mgr.post_turn("test task").unwrap();
        assert!(!hash2.is_empty());
        assert_ne!(hash1, hash2);

        let entries = mgr.list(10).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].message.contains("post-turn"));
        assert!(entries[1].message.contains("pre-turn"));
    }

    #[test]
    fn restore_reverts_file_content() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(workspace.join("data.txt"), "original").unwrap();
        let config_dir = tmp.path().join("lp");
        let mgr = SnapshotManager::new(&workspace, &config_dir);

        let hash = mgr.pre_turn("modify data").unwrap();

        // Modify the file
        std::fs::write(workspace.join("data.txt"), "modified").unwrap();
        mgr.post_turn("modify data").unwrap();

        // Restore to pre-turn state
        mgr.restore(&hash).unwrap();

        let content = std::fs::read_to_string(workspace.join("data.txt")).unwrap();
        assert_eq!(content, "original");
    }
}
