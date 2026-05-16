use crate::app::AppMode;
use crate::sandbox::Sandbox;
use crate::util::diff::generate_unified_diff;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub struct FileOps<'a> {
    sandbox: &'a Sandbox,
    mode: AppMode,
}

#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub action: FileAction,
    pub diff_preview: String,
}

#[derive(Debug, Clone)]
pub enum FileAction {
    Create { content: String },
    Modify { old_content: String, new_content: String },
    Delete,
}

impl<'a> FileOps<'a> {
    pub fn new(sandbox: &'a Sandbox, mode: AppMode) -> Self {
        Self { sandbox, mode }
    }

    pub fn read_file(&self, path: &Path) -> Result<String> {
        let _validated = self.sandbox.validate_path(path)?;
        std::fs::read_to_string(path)
            .with_context(|| format!("Reading {}", path.display()))
    }

    pub fn prepare_write(&self, path: &Path, content: &str) -> Result<FileChange> {
        self.sandbox.validate_path(path)?;

        if !self.mode.can_write_file() {
            anyhow::bail!("Current mode does not allow file writes");
        }

        let old_content = if path.exists() {
            std::fs::read_to_string(path).unwrap_or_default()
        } else {
            String::new()
        };

        let action = if old_content.is_empty() {
            FileAction::Create { content: content.to_string() }
        } else {
            FileAction::Modify {
                old_content: old_content.clone(),
                new_content: content.to_string(),
            }
        };

        let diff = generate_unified_diff(&old_content, content, &path.to_string_lossy());

        Ok(FileChange {
            path: path.to_path_buf(),
            action,
            diff_preview: diff,
        })
    }

    pub fn apply_change(&self, change: &FileChange) -> Result<()> {
        self.sandbox.validate_path(&change.path)?;
        if !self.mode.can_write_file() {
            anyhow::bail!("Current mode does not allow file writes");
        }

        match &change.action {
            FileAction::Create { content } | FileAction::Modify { new_content: content, .. } => {
                if let Some(parent) = change.path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&change.path, content)
                    .with_context(|| format!("Writing {}", change.path.display()))?;
            }
            FileAction::Delete => {
                if change.path.exists() {
                    std::fs::remove_file(&change.path)
                        .with_context(|| format!("Deleting {}", change.path.display()))?;
                }
            }
        }
        Ok(())
    }

    pub fn prepare_delete(&self, path: &Path) -> Result<FileChange> {
        self.sandbox.validate_path(path)?;
        if !self.mode.can_write_file() {
            anyhow::bail!("Current mode does not allow file deletion");
        }
        Ok(FileChange {
            path: path.to_path_buf(),
            action: FileAction::Delete,
            diff_preview: format!("--- DELETED: {}", path.display()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn plan_mode_blocks_write() {
        let dir = TempDir::new().unwrap();
        let sandbox = Sandbox::new(dir.path().to_path_buf());
        let ops = FileOps::new(&sandbox, AppMode::Plan);
        let result = ops.prepare_write(&dir.path().join("test.rs"), "fn main(){}");
        assert!(result.is_err());
    }

    #[test]
    fn edit_mode_allows_prepare_write() {
        let dir = TempDir::new().unwrap();
        let sandbox = Sandbox::new(dir.path().to_path_buf());
        let ops = FileOps::new(&sandbox, AppMode::Edit);
        let result = ops.prepare_write(&dir.path().join("test.rs"), "fn main(){}");
        assert!(result.is_ok());
    }

    #[test]
    fn auto_mode_applies_write() {
        let dir = TempDir::new().unwrap();
        let sandbox = Sandbox::new(dir.path().to_path_buf());
        let ops = FileOps::new(&sandbox, AppMode::Auto);
        let change = ops.prepare_write(&dir.path().join("test.rs"), "fn main(){}").unwrap();
        ops.apply_change(&change).unwrap();
        assert!(dir.path().join("test.rs").exists());
        let content = std::fs::read_to_string(dir.path().join("test.rs")).unwrap();
        assert_eq!(content, "fn main(){}");
    }

    #[test]
    fn path_outside_workspace_rejected() {
        let dir = TempDir::new().unwrap();
        let sandbox = Sandbox::new(dir.path().to_path_buf());
        let ops = FileOps::new(&sandbox, AppMode::Auto);
        let result = ops.prepare_write(Path::new("/tmp/outside.rs"), "bad");
        assert!(result.is_err());
    }

    #[test]
    fn delete_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("to_delete.txt");
        std::fs::write(&file, "content").unwrap();
        let sandbox = Sandbox::new(dir.path().to_path_buf());
        let ops = FileOps::new(&sandbox, AppMode::Auto);
        let change = ops.prepare_delete(&file).unwrap();
        ops.apply_change(&change).unwrap();
        assert!(!file.exists());
    }
}
