pub mod file_ops;
pub mod uv;

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FileEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub depth: usize,
    pub name: String,
}

#[allow(dead_code)]
pub struct ProjectContext {
    root: PathBuf,
    gitignore_patterns: Vec<String>,
}

#[allow(dead_code)]
impl ProjectContext {
    pub fn new(root: PathBuf) -> Self {
        let gitignore = Self::load_gitignore(&root);
        Self {
            root,
            gitignore_patterns: gitignore,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn load_gitignore(root: &Path) -> Vec<String> {
        let gi_path = root.join(".gitignore");
        if let Ok(content) = std::fs::read_to_string(&gi_path) {
            content
                .lines()
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(|l| l.to_string())
                .collect()
        } else {
            vec![
                "target".into(),
                "node_modules".into(),
                ".git".into(),
                "__pycache__".into(),
                ".venv".into(),
                "venv".into(),
            ]
        }
    }

    fn is_ignored(&self, path: &Path) -> bool {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        for pattern in &self.gitignore_patterns {
            if name == pattern || path.to_str().unwrap_or("").contains(pattern) {
                return true;
            }
        }
        false
    }

    pub fn list_tree(&self) -> Vec<FileEntry> {
        let mut entries = Vec::new();
        for entry in WalkDir::new(&self.root)
            .max_depth(5)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let path = e.path();
                !self.is_ignored(path)
            })
        {
            let Ok(entry) = entry else { continue };
            let path = entry.path().to_path_buf();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().is_dir();
            let depth = entry.depth();
            entries.push(FileEntry {
                path,
                is_dir,
                depth,
                name,
            });
        }
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn list_tree_basic() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("main.rs"), "fn main(){}").unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "").unwrap();

        let ctx = ProjectContext::new(dir.path().to_path_buf());
        let tree = ctx.list_tree();
        let names: Vec<&str> = tree.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"main.rs"));
        assert!(names.contains(&"lib.rs"));
    }

    #[test]
    fn gitignore_excludes_target() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("target/debug")).unwrap();
        std::fs::write(dir.path().join("target/debug/build"), "").unwrap();
        std::fs::write(dir.path().join(".gitignore"), "target\n").unwrap();

        let ctx = ProjectContext::new(dir.path().to_path_buf());
        let tree = ctx.list_tree();
        let names: Vec<&str> = tree.iter().map(|e| e.name.as_str()).collect();
        assert!(!names.contains(&"build"));
    }
}
