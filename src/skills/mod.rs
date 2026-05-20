pub mod builtin;
pub mod parser;

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub trigger: String,
    pub content: String,
}

pub struct SkillRegistry {
    skills: Vec<Skill>,
}

impl SkillRegistry {
    pub fn load_from_dir(dir: &Path) -> Self {
        let mut skills = Vec::new();

        if !dir.exists() {
            return Self { skills };
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Some(skill) = parser::parse_skill(&content) {
                            skills.push(skill);
                        }
                    }
                }
            }
        }

        Self { skills }
    }

    pub fn empty() -> Self {
        Self { skills: Vec::new() }
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.name == name)
    }

    pub fn list(&self) -> &[Skill] {
        &self.skills
    }

    #[allow(dead_code)]
    pub fn match_trigger(&self, input: &str) -> Option<&Skill> {
        let input_lower = input.to_lowercase();
        self.skills.iter().find(|s| {
            s.trigger
                .split(',')
                .any(|t| input_lower.contains(&t.trim().to_lowercase()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_skill(name: &str, desc: &str, trigger: &str) -> Skill {
        Skill {
            name: name.to_string(),
            description: desc.to_string(),
            trigger: trigger.to_string(),
            content: "skill body".to_string(),
        }
    }

    #[test]
    fn get_skill_by_name() {
        let registry = SkillRegistry {
            skills: vec![
                make_skill("review", "Review code", "review"),
                make_skill("search", "Search files", "search, find"),
            ],
        };
        assert!(registry.get("review").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn match_trigger_keyword() {
        let registry = SkillRegistry {
            skills: vec![
                make_skill("review", "Review code", "code review, review"),
                make_skill("search", "Search files", "search, find knowledge"),
            ],
        };
        assert!(registry.match_trigger("please code review this").is_some());
        assert!(registry.match_trigger("find knowledge about X").is_some());
        assert!(registry.match_trigger("random unrelated text").is_none());
    }

    #[test]
    fn empty_registry() {
        let registry = SkillRegistry::empty();
        assert!(registry.list().is_empty());
        assert!(registry.get("anything").is_none());
    }
}
