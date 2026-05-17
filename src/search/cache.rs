use super::SearchResult;
use std::fs;
use std::path::PathBuf;

#[allow(dead_code)]
pub struct SearchCache {
    cache_dir: PathBuf,
    valid_days: u64,
}

#[allow(dead_code)]
impl SearchCache {
    pub fn new(cache_dir: PathBuf, valid_days: u64) -> Self {
        let _ = fs::create_dir_all(&cache_dir);
        Self { cache_dir, valid_days }
    }

    pub fn get(&self, query: &str) -> Option<Vec<SearchResult>> {
        let path = self.cache_path(query);
        if !path.exists() {
            return None;
        }

        let metadata = fs::metadata(&path).ok()?;
        let modified = metadata.modified().ok()?;
        let age = modified.elapsed().ok()?;
        let max_age = std::time::Duration::from_secs(self.valid_days * 86400);

        if age > max_age {
            let _ = fs::remove_file(&path);
            return None;
        }

        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn set(&self, query: &str, results: &[SearchResult]) {
        let path = self.cache_path(query);
        if let Ok(json) = serde_json::to_string_pretty(results) {
            let _ = fs::write(&path, json);
        }
    }

    fn cache_path(&self, query: &str) -> PathBuf {
        let hash = Self::hash_query(query);
        self.cache_dir.join(format!("{}.json", hash))
    }

    fn hash_query(query: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        query.to_lowercase().hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_result(title: &str) -> SearchResult {
        SearchResult {
            title: title.to_string(),
            url: format!("http://{}.com", title),
            snippet: "test snippet".to_string(),
            body: "test body".to_string(),
        }
    }

    #[test]
    fn cache_miss_returns_none() {
        let dir = TempDir::new().unwrap();
        let cache = SearchCache::new(dir.path().to_path_buf(), 30);
        assert!(cache.get("nonexistent query").is_none());
    }

    #[test]
    fn cache_set_and_get() {
        let dir = TempDir::new().unwrap();
        let cache = SearchCache::new(dir.path().to_path_buf(), 30);
        let results = vec![make_result("rust-lang")];
        cache.set("rust programming", &results);
        let cached = cache.get("rust programming");
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].title, "rust-lang");
    }

    #[test]
    fn cache_hash_is_case_insensitive() {
        let hash1 = SearchCache::hash_query("Rust Programming");
        let hash2 = SearchCache::hash_query("rust programming");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn cache_expiry() {
        let dir = TempDir::new().unwrap();
        let cache = SearchCache::new(dir.path().to_path_buf(), 0); // 0 days = expired immediately
        let results = vec![make_result("test")];
        cache.set("test query", &results);
        // With 0 valid_days, cache should be expired
        assert!(cache.get("test query").is_none());
    }
}
