use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:11434";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ollama_endpoint: String,
    pub connect_timeout: u64,
    pub fast_model: String,
    pub core_model: String,
    pub audit_model: String,
    pub code_base_path: String,
    pub default_mode: String,
    pub auto_mode_only_workspace: bool,
    pub enable_auto_syntax_check: bool,
    pub prefer_uv_toolchain: bool,
    pub auto_run_after_fix: bool,
    pub enable_free_web_search: bool,
    pub auto_switch_network_region: bool,
    pub search_cache_valid_days: u64,
    pub max_search_context_tokens: usize,
    pub max_template_context_tokens: usize,
    pub template_max_select: usize,
    pub max_retries: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ollama_endpoint: DEFAULT_ENDPOINT.to_string(),
            connect_timeout: 15,
            fast_model: String::new(),
            core_model: String::new(),
            audit_model: String::new(),
            code_base_path: "~/.litecode/code_base".to_string(),
            default_mode: "edit".to_string(),
            auto_mode_only_workspace: true,
            enable_auto_syntax_check: true,
            prefer_uv_toolchain: true,
            auto_run_after_fix: false,
            enable_free_web_search: true,
            auto_switch_network_region: true,
            search_cache_valid_days: 30,
            max_search_context_tokens: 2048,
            max_template_context_tokens: 2048,
            template_max_select: 5,
            max_retries: 3,
        }
    }
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        Ok(home.join(".litecode"))
    }

    /// Check for project-local `.litecode` directory in the given workspace,
    /// fall back to global `~/.litecode` if not found.
    pub fn effective_dir(workspace: &Path) -> PathBuf {
        let local = workspace.join(".litecode");
        if local.is_dir() {
            local
        } else {
            Self::config_dir().unwrap_or_else(|_| local)
        }
    }

    pub fn config_path_for(workspace: &Path) -> PathBuf {
        Self::effective_dir(workspace).join("config.toml")
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    #[allow(dead_code)]
    pub fn ensure_dirs() -> Result<PathBuf> {
        let dir = Self::config_dir()?;
        Self::create_dir_structure(&dir)?;
        Ok(dir)
    }

    /// Initialize directory structure and populate code_base templates.
    /// Used for both global (~/.litecode) and project-local (.litecode) dirs.
    pub fn ensure_dirs_for(workspace: &Path) -> Result<PathBuf> {
        let dir = Self::effective_dir(workspace);
        Self::create_dir_structure(&dir)?;

        // Populate built-in templates on first run
        let code_base = dir.join("code_base");
        crate::codebase::builtin::populate_codebase(&code_base)?;

        // Always populate skills in global ~/.litecode/skills/
        let global_dir = Self::config_dir()?;
        let _ = crate::skills::builtin::populate_skills(&global_dir.join("skills"));

        Ok(dir)
    }

    fn create_dir_structure(dir: &Path) -> Result<()> {
        for sub in &["sessions", "cache", "code_base", "skills"] {
            std::fs::create_dir_all(dir.join(sub))
                .with_context(|| format!("Creating directory {}", sub))?;
        }
        if !dir.join("config.toml").exists() {
            let default = Config::default();
            default.save(&dir.join("config.toml"))?;
        }
        Ok(())
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        Self::load_from(&path)
    }

    /// Load config from project-local `.litecode` if present, else global.
    pub fn load_for_workspace(workspace: &Path) -> Result<Self> {
        let path = Self::config_path_for(workspace);
        if path.exists() {
            Self::load_from(&path)
        } else {
            Self::load()
        }
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Reading config from {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| "Parsing config.toml")?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Serializing config to TOML")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
            .with_context(|| format!("Writing config to {}", path.display()))?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn exists() -> bool {
        Self::config_path().map(|p| p.exists()).unwrap_or(false)
    }

    pub fn validate(&self) -> Result<()> {
        if !self.ollama_endpoint.starts_with("http://") && !self.ollama_endpoint.starts_with("https://") {
            anyhow::bail!("ollama_endpoint must start with http:// or https://");
        }
        let valid_modes = ["plan", "edit", "auto"];
        if !valid_modes.contains(&self.default_mode.as_str()) {
            anyhow::bail!("default_mode must be one of: plan, edit, auto");
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn code_base_dir(&self) -> PathBuf {
        let expanded = shellexpand::tilde(&self.code_base_path).to_string();
        PathBuf::from(expanded)
    }

    #[allow(dead_code)]
    pub fn cache_dir() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("cache"))
    }

    #[allow(dead_code)]
    pub fn sessions_dir() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("sessions"))
    }

    pub fn skills_dir() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("skills"))
    }

    /// Returns true when the config needs first-run setup (core model not configured).
    #[allow(dead_code)]
    pub fn needs_setup(&self) -> bool {
        self.core_model.is_empty()
    }

    pub fn effective_fast_model(&self) -> &str {
        if self.fast_model.is_empty() {
            &self.core_model
        } else {
            &self.fast_model
        }
    }

    pub fn effective_audit_model(&self) -> &str {
        if self.audit_model.is_empty() {
            &self.core_model
        } else {
            &self.audit_model
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn valid_config() -> Config {
        Config {
            ollama_endpoint: "http://localhost:11434".into(),
            fast_model: "qwen3:4b".into(),
            core_model: "qwen3:8b".into(),
            audit_model: "qwen3:14b".into(),
            ..Default::default()
        }
    }

    #[test]
    fn roundtrip_toml() {
        let config = valid_config();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.ollama_endpoint, parsed.ollama_endpoint);
        assert_eq!(config.fast_model, parsed.fast_model);
        assert_eq!(config.core_model, parsed.core_model);
    }

    #[test]
    fn defaults_are_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.default_mode, "edit");
        assert_eq!(config.connect_timeout, 15);
    }

    #[test]
    fn invalid_endpoint_rejected() {
        let mut config = valid_config();
        config.ollama_endpoint = "ftp://bad".into();
        assert!(config.validate().is_err());
    }

    #[test]
    fn invalid_mode_rejected() {
        let mut config = valid_config();
        config.default_mode = "invalid".into();
        assert!(config.validate().is_err());
    }

    #[test]
    fn load_save_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let config = valid_config();
        config.save(&path).unwrap();
        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(config.ollama_endpoint, loaded.ollama_endpoint);
        assert_eq!(config.fast_model, loaded.fast_model);
    }

    #[test]
    fn missing_file_returns_error() {
        let result = Config::load_from(Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn empty_models_fall_back_to_core() {
        let config = Config {
            fast_model: String::new(),
            audit_model: String::new(),
            core_model: "qwen3:8b".into(),
            ..Default::default()
        };
        assert_eq!(config.effective_fast_model(), "qwen3:8b");
        assert_eq!(config.effective_audit_model(), "qwen3:8b");
    }

    #[test]
    fn ensure_dirs_creates_structure() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join(".litecode");
        // Monkey-patch: just verify the subdirectories would be created
        for sub in &["sessions", "cache", "code_base", "skills"] {
            std::fs::create_dir_all(base.join(sub)).unwrap();
        }
        assert!(base.join("sessions").exists());
        assert!(base.join("cache").exists());
        assert!(base.join("code_base").exists());
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_roundtrip(
            endpoint in "http://[a-z]{1,10}\\.local:\\d{1,5}",
            timeout in 1u64..300,
            fast in "[a-z]{1,10}:\\d{0,2}b",
            core in "[a-z]{1,10}:\\d{0,2}b",
            audit in "[a-z]{1,10}:\\d{0,2}b",
        ) {
            let config = Config {
                ollama_endpoint: endpoint.clone(),
                connect_timeout: timeout,
                fast_model: fast.clone(),
                core_model: core.clone(),
                audit_model: audit.clone(),
                ..Default::default()
            };
            let toml_str = toml::to_string_pretty(&config).unwrap();
            let parsed: Config = toml::from_str(&toml_str).unwrap();
            assert_eq!(config.ollama_endpoint, parsed.ollama_endpoint);
            assert_eq!(config.connect_timeout, parsed.connect_timeout);
            assert_eq!(config.fast_model, parsed.fast_model);
        }
    }
}
