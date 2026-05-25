use std::cmp::Reverse;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

const MAX_ENTRIES: usize = 20;

/// Per-file tracking data for frecency scoring.
#[derive(Debug, Clone)]
struct FileEntry {
    path: String,
    access_count: usize,
    last_access: Instant,
}

/// Tracks files the agent touches (reads, writes, edits) using frecency ranking.
/// Injected into the system prompt so the LLM knows its active working context.
pub struct WorkingSet {
    entries: HashMap<String, FileEntry>,
}

impl WorkingSet {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Record that a file was accessed (read, written, or edited).
    pub fn touch(&mut self, path: &Path) {
        let key = path.to_string_lossy().to_string();
        let now = Instant::now();
        self.entries
            .entry(key.clone())
            .and_modify(|e| {
                e.access_count += 1;
                e.last_access = now;
            })
            .or_insert(FileEntry {
                path: key,
                access_count: 1,
                last_access: now,
            });

        self.prune();
    }

    /// Remove entries beyond MAX_ENTRIES, keeping highest frecency scores.
    fn prune(&mut self) {
        if self.entries.len() <= MAX_ENTRIES {
            return;
        }
        let mut scored: Vec<_> = self.entries.values().collect();
        scored.sort_by_key(|e| Reverse(e.frecency_score()));
        let keep: std::collections::HashSet<String> = scored
            .into_iter()
            .take(MAX_ENTRIES)
            .map(|e| e.path.clone())
            .collect();
        self.entries.retain(|k, _| keep.contains(k));
    }

    /// Generate a summary for injection into the system prompt.
    /// Returns None if no files are tracked.
    pub fn summary(&self) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }

        let mut sorted: Vec<_> = self.entries.values().collect();
        sorted.sort_by_key(|e| Reverse(e.frecency_score()));

        let paths: Vec<&str> = sorted.iter().map(|e| e.path.as_str()).collect();
        Some(format!("Active files: {}", paths.join(", ")))
    }

    /// Get the list of tracked paths, ordered by frecency.
    #[allow(dead_code)]
    pub fn paths(&self) -> Vec<String> {
        let mut sorted: Vec<_> = self.entries.values().collect();
        sorted.sort_by_key(|e| Reverse(e.frecency_score()));
        sorted.iter().map(|e| e.path.clone()).collect()
    }

    /// Number of tracked files.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether any files are tracked.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all tracked files.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl FileEntry {
    /// Frecency score: frequency * recency weight.
    /// More recent accesses score higher.
    fn frecency_score(&self) -> u64 {
        let elapsed_secs = self.last_access.elapsed().as_secs();
        let recency_weight = if elapsed_secs < 60 {
            4
        } else if elapsed_secs < 300 {
            3
        } else if elapsed_secs < 1800 {
            2
        } else {
            1
        };
        (self.access_count as u64) * recency_weight
    }
}

impl Default for WorkingSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn new_is_empty() {
        let ws = WorkingSet::new();
        assert!(ws.is_empty());
        assert!(ws.summary().is_none());
    }

    #[test]
    fn touch_adds_file() {
        let mut ws = WorkingSet::new();
        ws.touch(Path::new("src/main.rs"));
        assert_eq!(ws.len(), 1);
        assert!(ws.summary().unwrap().contains("src/main.rs"));
    }

    #[test]
    fn multiple_touches_increase_count() {
        let mut ws = WorkingSet::new();
        let path = PathBuf::from("src/main.rs");
        ws.touch(&path);
        ws.touch(&path);
        ws.touch(&path);
        assert_eq!(ws.len(), 1);
        let summary = ws.summary().unwrap();
        assert!(summary.contains("src/main.rs"));
    }

    #[test]
    fn summary_orders_by_frecency() {
        let mut ws = WorkingSet::new();
        let frequent = PathBuf::from("src/main.rs");
        let rare = PathBuf::from("README.md");

        // Touch rare once
        ws.touch(&rare);
        // Touch frequent many times
        for _ in 0..5 {
            ws.touch(&frequent);
        }

        let paths = ws.paths();
        assert_eq!(paths[0], "src/main.rs");
        assert_eq!(paths[1], "README.md");
    }

    #[test]
    fn prune_keeps_max_entries() {
        let mut ws = WorkingSet::new();
        for i in 0..30 {
            ws.touch(Path::new(&format!("file_{}.rs", i)));
        }
        assert_eq!(ws.len(), MAX_ENTRIES);
    }

    #[test]
    fn clear_empties_set() {
        let mut ws = WorkingSet::new();
        ws.touch(Path::new("src/main.rs"));
        ws.clear();
        assert!(ws.is_empty());
    }

    #[test]
    fn paths_returns_sorted() {
        let mut ws = WorkingSet::new();
        ws.touch(Path::new("b.rs"));
        ws.touch(Path::new("a.rs"));
        ws.touch(Path::new("a.rs")); // a.rs accessed twice

        let paths = ws.paths();
        assert_eq!(paths[0], "a.rs");
        assert_eq!(paths[1], "b.rs");
    }
}
