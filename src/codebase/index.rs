use super::Template;
use regex::Regex;
use std::fs;
use std::path::Path;

pub fn scan_directory(dir: &Path) -> Result<Vec<Template>, std::io::Error> {
    let mut templates = Vec::new();
    if !dir.exists() {
        return Ok(templates);
    }

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "py" | "js" | "ts" | "rs" | "go" | "html" | "css" | "sh" | "toml" | "json") {
            continue;
        }

        if let Ok(content) = fs::read_to_string(path) {
            if let Some(template) = parse_template(content, path, dir) {
                templates.push(template);
            }
        }
    }

    Ok(templates)
}

fn parse_template(content: String, path: &Path, base_dir: &Path) -> Option<Template> {
    let desc_re = Regex::new(r"(?:#|//|<!--|--)\s*@LITE_DESC:\s*(.+)").ok()?;
    let scene_re = Regex::new(r"(?:#|//|<!--|--)\s*@LITE_SCENE:\s*(.+)").ok()?;
    let tags_re = Regex::new(r"(?:#|//|<!--|--)\s*@LITE_TAGS:\s*(.+)").ok()?;

    let desc = desc_re
        .captures(&content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_default();

    let scene = scene_re
        .captures(&content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_default();

    let tags_str = tags_re
        .captures(&content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_default();

    let tags: Vec<String> = tags_str
        .split(|c: char| c == ',' || c == ' ')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    let name = path.file_stem()?.to_str()?.to_string();
    let rel_path = path.strip_prefix(base_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    Some(Template {
        name,
        path: rel_path,
        description: desc,
        scene,
        tags,
        content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_template(dir: &Path, name: &str, desc: &str, scene: &str, tags: &str, body: &str) {
        let content = format!(
            "# @LITE_DESC: {}\n# @LITE_SCENE: {}\n# @LITE_TAGS: {}\n{}",
            desc, scene, tags, body
        );
        fs::write(dir.join(name), content).unwrap();
    }

    #[test]
    fn scan_finds_tagged_files() {
        let dir = TempDir::new().unwrap();
        write_template(
            dir.path(), "app.py",
            "Flask app", "web development", "python, flask, web",
            "from flask import Flask\napp = Flask(__name__)\n",
        );
        let templates = scan_directory(dir.path()).unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "app");
        assert_eq!(templates[0].description, "Flask app");
        assert!(templates[0].tags.contains(&"flask".to_string()));
    }

    #[test]
    fn scan_skips_untagged_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("plain.py"), "print('hello')\n").unwrap();
        let templates = scan_directory(dir.path()).unwrap();
        // Should still parse but with empty desc/tags
        assert_eq!(templates.len(), 1);
        assert!(templates[0].description.is_empty());
    }

    #[test]
    fn scan_empty_dir() {
        let dir = TempDir::new().unwrap();
        let templates = scan_directory(dir.path()).unwrap();
        assert!(templates.is_empty());
    }

    #[test]
    fn scan_nonexistent_dir() {
        let templates = scan_directory(Path::new("/nonexistent/path"));
        assert!(templates.is_ok());
        assert!(templates.unwrap().is_empty());
    }
}
