/// Parse a skill from markdown content with YAML-like frontmatter.
///
/// Expected format:
/// ```markdown
/// ---
/// name: skill-name
/// description: One-line description
/// trigger: keyword1, keyword2
/// ---
///
/// Skill body content here...
/// ```
pub fn parse_skill(content: &str) -> Option<super::Skill> {
    let trimmed = content.trim();

    if !trimmed.starts_with("---") {
        return None;
    }

    let rest = trimmed.strip_prefix("---")?;

    let end = rest.find("---")?;
    let frontmatter = &rest[..end];
    let body = rest[end + 3..].trim();

    let mut name = String::new();
    let mut description = String::new();
    let mut trigger = String::new();

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("name:") {
            name = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("description:") {
            description = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("trigger:") {
            trigger = val.trim().to_string();
        }
    }

    if name.is_empty() {
        return None;
    }

    Some(super::Skill {
        name,
        description,
        trigger,
        content: body.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_skill() {
        let content = "\
---
name: review
description: Review code for bugs
trigger: code review, review
---

You are a code reviewer.
Analyze the code for bugs and style issues.";
        let skill = parse_skill(content).unwrap();
        assert_eq!(skill.name, "review");
        assert_eq!(skill.description, "Review code for bugs");
        assert_eq!(skill.trigger, "code review, review");
        assert!(skill.content.starts_with("You are a code reviewer."));
    }

    #[test]
    fn parse_skill_no_frontmatter() {
        let content = "Just some regular markdown without frontmatter.";
        assert!(parse_skill(content).is_none());
    }

    #[test]
    fn parse_skill_missing_name() {
        let content = "\
---
description: Missing name field
---
Body";
        assert!(parse_skill(content).is_none());
    }

    #[test]
    fn parse_skill_minimal() {
        let content = "\
---
name: test
---
Body content";
        let skill = parse_skill(content).unwrap();
        assert_eq!(skill.name, "test");
        assert!(skill.description.is_empty());
        assert!(skill.trigger.is_empty());
        assert_eq!(skill.content, "Body content");
    }
}
