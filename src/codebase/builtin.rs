use std::fs;
use std::path::Path;

/// Returns all built-in templates as (relative_path, content) pairs.
/// Each template file has @LIGHT_DESC / @LIGHT_SCENE / @LIGHT_TAGS headers
/// for lightweight RAG matching by the codebase index.
pub fn all_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        // ── LaTeX ──────────────────────────────────────────────
        ("latex/article.tex", include_str!("templates/latex/article.tex")),
        ("latex/beamer.tex", include_str!("templates/latex/beamer.tex")),
        ("latex/cv.tex", include_str!("templates/latex/cv.tex")),
        ("latex/ieee_paper.tex", include_str!("templates/latex/ieee_paper.tex")),
        // ── Markdown ───────────────────────────────────────────
        ("markdown/readme.md", include_str!("templates/markdown/readme.md")),
        ("markdown/changelog.md", include_str!("templates/markdown/changelog.md")),
        ("markdown/api_docs.md", include_str!("templates/markdown/api_docs.md")),
        ("markdown/contributing.md", include_str!("templates/markdown/contributing.md")),
        // ── HTML ───────────────────────────────────────────────
        ("html/basic_page.html", include_str!("templates/html/basic_page.html")),
        ("html/responsive_landing.html", include_str!("templates/html/responsive_landing.html")),
        ("html/contact_form.html", include_str!("templates/html/contact_form.html")),
        ("html/dashboard.html", include_str!("templates/html/dashboard.html")),
        // ── CSS ────────────────────────────────────────────────
        ("css/reset.css", include_str!("templates/css/reset.css")),
        ("css/responsive_grid.css", include_str!("templates/css/responsive_grid.css")),
        ("css/dark_theme.css", include_str!("templates/css/dark_theme.css")),
        // ── Python ─────────────────────────────────────────────
        ("python/flask_api.py", include_str!("templates/python/flask_api.py")),
        ("python/cli_tool.py", include_str!("templates/python/cli_tool.py")),
        ("python/data_processing.py", include_str!("templates/python/data_processing.py")),
        ("python/pytest_template.py", include_str!("templates/python/pytest_template.py")),
        ("python/fastapi_app.py", include_str!("templates/python/fastapi_app.py")),
        ("python/web_scraper.py", include_str!("templates/python/web_scraper.py")),
        // ── JavaScript / TypeScript ────────────────────────────
        ("javascript/express_server.js", include_str!("templates/javascript/express_server.js")),
        ("javascript/dom_manipulation.js", include_str!("templates/javascript/dom_manipulation.js")),
        ("javascript/fetch_api.js", include_str!("templates/javascript/fetch_api.js")),
        ("javascript/node_cli.js", include_str!("templates/javascript/node_cli.js")),
        ("typescript/express_ts.ts", include_str!("templates/typescript/express_ts.ts")),
        ("typescript/type_definitions.ts", include_str!("templates/typescript/type_definitions.ts")),
        // ── Rust ───────────────────────────────────────────────
        ("rust/cli_app.rs", include_str!("templates/rust/cli_app.rs")),
        ("rust/axum_server.rs", include_str!("templates/rust/axum_server.rs")),
        ("rust/lib_skeleton.rs", include_str!("templates/rust/lib_skeleton.rs")),
        // ── Go ─────────────────────────────────────────────────
        ("go/http_server.go", include_str!("templates/go/http_server.go")),
        ("go/cli_tool.go", include_str!("templates/go/cli_tool.go")),
        // ── Shell ──────────────────────────────────────────────
        ("shell/backup_script.sh", include_str!("templates/shell/backup_script.sh")),
        ("shell/env_setup.sh", include_str!("templates/shell/env_setup.sh")),
        ("shell/git_helpers.sh", include_str!("templates/shell/git_helpers.sh")),
        // ── SQL ────────────────────────────────────────────────
        ("sql/schema.sql", include_str!("templates/sql/schema.sql")),
        ("sql/crud_queries.sql", include_str!("templates/sql/crud_queries.sql")),
        // ── Docker ─────────────────────────────────────────────
        ("docker/Dockerfile", include_str!("templates/docker/Dockerfile")),
        ("docker/Dockerfile_python", include_str!("templates/docker/Dockerfile_python")),
        ("docker/Dockerfile_node", include_str!("templates/docker/Dockerfile_node")),
        ("docker/docker_compose.yml", include_str!("templates/docker/docker_compose.yml")),
        // ── C / C++ ────────────────────────────────────────────
        ("c_basic_program.c", include_str!("templates/c/basic_program.c")),
        ("cpp/basic_program.cpp", include_str!("templates/cpp/basic_program.cpp")),
        ("cpp/class_template.hpp", include_str!("templates/cpp/class_template.hpp")),
        // ── Config / CI ────────────────────────────────────────
        ("config/cargo_toml.toml", include_str!("templates/config/cargo_toml.toml")),
        ("config/github_actions_ci.yml", include_str!("templates/config/github_actions_ci.yml")),
        ("config/gitignore", include_str!("templates/config/gitignore")),
        ("config/env_example.env", include_str!("templates/config/env_example.env")),
        // ── Java ───────────────────────────────────────────────
        ("java/basic_app.java", include_str!("templates/java/basic_app.java")),
        ("java/spring_controller.java", include_str!("templates/java/spring_controller.java")),
        // ── Lua ────────────────────────────────────────────────
        ("lua/basic_script.lua", include_str!("templates/lua/basic_script.lua")),
        // ── Zig ────────────────────────────────────────────────
        ("zig/basic_program.zig", include_str!("templates/zig/basic_program.zig")),
    ]
}

/// Write all built-in templates to `~/.litecode/code_base/`.
/// Skips files that already exist (preserves user customizations).
pub fn populate_codebase(base_dir: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(base_dir)?;
    for (rel_path, content) in all_templates() {
        let full_path = base_dir.join(rel_path);
        if full_path.exists() {
            continue; // never overwrite user modifications
        }
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&full_path, content)?;
    }
    Ok(())
}

/// Count of built-in templates (for status display).
pub fn template_count() -> usize {
    all_templates().len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn all_templates_are_nonempty() {
        for (path, content) in all_templates() {
            assert!(!content.is_empty(), "Template {} is empty", path);
            assert!(content.contains("@LITE_DESC"), "Template {} missing @LITE_DESC", path);
            assert!(content.contains("@LITE_TAGS"), "Template {} missing @LITE_TAGS", path);
        }
    }

    #[test]
    fn populate_creates_files() {
        let dir = TempDir::new().unwrap();
        populate_codebase(dir.path()).unwrap();
        let count = walkdir::WalkDir::new(dir.path())
            .into_iter()
            .filter(|e| e.as_ref().map(|e| e.file_type().is_file()).unwrap_or(false))
            .count();
        assert_eq!(count, all_templates().len());
    }

    #[test]
    fn populate_does_not_overwrite() {
        let dir = TempDir::new().unwrap();
        let sentinel = dir.path().join("latex/article.tex");
        fs::create_dir_all(sentinel.parent().unwrap()).unwrap();
        fs::write(&sentinel, "USER CUSTOM CONTENT").unwrap();

        populate_codebase(dir.path()).unwrap();

        let content = fs::read_to_string(&sentinel).unwrap();
        assert_eq!(content, "USER CUSTOM CONTENT");
    }

    #[test]
    fn template_count_matches() {
        assert_eq!(template_count(), all_templates().len());
        assert!(template_count() > 40);
    }
}
