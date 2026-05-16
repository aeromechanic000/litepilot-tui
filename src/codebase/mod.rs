pub mod builtin;
pub mod index;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    pub path: String,
    pub description: String,
    pub scene: String,
    pub tags: Vec<String>,
    pub content: String,
}

pub struct CodeBase {
    templates: Vec<Template>,
    base_dir: PathBuf,
}

impl CodeBase {
    pub fn new(base_dir: PathBuf) -> Self {
        let mut cb = Self {
            templates: Vec::new(),
            base_dir,
        };
        let _ = cb.load_templates();
        cb
    }

    fn load_templates(&mut self) -> Result<(), std::io::Error> {
        if !self.base_dir.exists() {
            return Ok(());
        }
        self.templates = index::scan_directory(&self.base_dir)?;
        Ok(())
    }

    pub fn templates(&self) -> &[Template] {
        &self.templates
    }

    pub fn search(&self, query: &str, tags: &[&str]) -> Vec<&Template> {
        let query_lower = query.to_lowercase();
        if query_lower.is_empty() && tags.is_empty() {
            return Vec::new();
        }
        self.templates
            .iter()
            .filter(|t| {
                let matches_query = !query_lower.is_empty() && (
                    t.description.to_lowercase().contains(&query_lower)
                    || t.name.to_lowercase().contains(&query_lower)
                    || t.scene.to_lowercase().contains(&query_lower)
                );
                let matches_tags = !tags.is_empty()
                    && tags.iter().any(|tag| {
                        t.tags.iter().any(|t_tag| t_tag.to_lowercase().contains(&tag.to_lowercase()))
                    });
                matches_query || matches_tags
            })
            .collect()
    }

    pub fn load_template_content(&self, name: &str) -> Option<&str> {
        self.templates
            .iter()
            .find(|t| t.name == name)
            .map(|t| t.content.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_template(name: &str, desc: &str, tags: Vec<&str>) -> Template {
        Template {
            name: name.to_string(),
            path: format!("templates/{}.py", name),
            description: desc.to_string(),
            scene: "web development".to_string(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            content: "# sample code".to_string(),
        }
    }

    #[test]
    fn search_by_description() {
        let cb = CodeBase {
            templates: vec![
                make_test_template("flask_app", "Flask web application template", vec!["python", "flask"]),
                make_test_template("snake_game", "Classic snake game", vec!["javascript", "game"]),
            ],
            base_dir: PathBuf::from("/tmp"),
        };
        let results = cb.search("flask", &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "flask_app");
    }

    #[test]
    fn search_by_tags() {
        let cb = CodeBase {
            templates: vec![
                make_test_template("flask_app", "Flask app", vec!["python", "flask"]),
                make_test_template("snake_game", "Snake game", vec!["javascript", "game"]),
            ],
            base_dir: PathBuf::from("/tmp"),
        };
        let results = cb.search("", &["game"]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "snake_game");
    }

    #[test]
    fn search_empty_query_returns_nothing() {
        let cb = CodeBase {
            templates: vec![make_test_template("test", "desc", vec!["tag"])],
            base_dir: PathBuf::from("/tmp"),
        };
        let results = cb.search("", &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn load_template_by_name() {
        let cb = CodeBase {
            templates: vec![make_test_template("flask_app", "desc", vec![])],
            base_dir: PathBuf::from("/tmp"),
        };
        assert!(cb.load_template_content("flask_app").is_some());
        assert!(cb.load_template_content("nonexistent").is_none());
    }
}
