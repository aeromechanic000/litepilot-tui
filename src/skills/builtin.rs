use anyhow::Result;
use std::path::Path;

const SEARCH_SKILL: &str = include_str!("../skills_builtin/search.md");
const REVIEW_SKILL: &str = include_str!("../skills_builtin/review.md");
const EXPLAIN_SKILL: &str = include_str!("../skills_builtin/explain.md");
const SIMPLIFY_SKILL: &str = include_str!("../skills_builtin/simplify.md");
const TEST_SKILL: &str = include_str!("../skills_builtin/test.md");

const BUILTIN_SKILLS: &[&str] = &[
    SEARCH_SKILL,
    REVIEW_SKILL,
    EXPLAIN_SKILL,
    SIMPLIFY_SKILL,
    TEST_SKILL,
];

pub fn populate_skills(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;

    for skill_content in BUILTIN_SKILLS {
        if let Some(skill) = super::parser::parse_skill(skill_content) {
            let path = dir.join(format!("{}.md", skill.name));
            if !path.exists() {
                std::fs::write(&path, skill_content)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn populate_creates_skill_files() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        populate_skills(&skills_dir).unwrap();

        assert!(skills_dir.join("search.md").exists());
        assert!(skills_dir.join("review.md").exists());
        assert!(skills_dir.join("explain.md").exists());
        assert!(skills_dir.join("simplify.md").exists());
        assert!(skills_dir.join("test.md").exists());
    }

    #[test]
    fn populate_does_not_overwrite() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let custom_content = "---\nname: review\n---\nMy custom review prompt";
        std::fs::write(skills_dir.join("review.md"), custom_content).unwrap();

        populate_skills(&skills_dir).unwrap();

        let content = std::fs::read_to_string(skills_dir.join("review.md")).unwrap();
        assert!(content.contains("My custom review prompt"));
    }
}
