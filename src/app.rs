use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppMode {
    Plan,
    Edit,
    Auto,
}

impl std::fmt::Display for AppMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppMode::Plan => write!(f, "PLAN"),
            AppMode::Edit => write!(f, "EDIT"),
            AppMode::Auto => write!(f, "AUTO"),
        }
    }
}

impl AppMode {
    pub fn cycle(self) -> Self {
        match self {
            AppMode::Plan => AppMode::Edit,
            AppMode::Edit => AppMode::Auto,
            AppMode::Auto => AppMode::Plan,
        }
    }

    pub fn can_write_file(self) -> bool {
        !matches!(self, AppMode::Plan)
    }

    pub fn can_execute_command(self) -> bool {
        !matches!(self, AppMode::Plan)
    }

    pub fn needs_confirmation(self) -> bool {
        matches!(self, AppMode::Edit)
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "plan" => AppMode::Plan,
            "auto" => AppMode::Auto,
            _ => AppMode::Edit,
        }
    }
}

pub struct AppState {
    pub mode: AppMode,
    pub config: Config,
    pub workspace: PathBuf,
    pub web_search_enabled: bool,
    pub pending_confirmations: Vec<PendingAction>,
    pub input_history: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum PendingAction {
    WriteFile { path: PathBuf, content: String, diff_preview: String },
    DeleteFile { path: PathBuf },
    ExecuteCommand { cmd: String, args: Vec<String> },
}

impl AppState {
    pub fn new(config: Config, workspace: PathBuf) -> Self {
        let mode = AppMode::from_str_lossy(&config.default_mode);
        Self {
            mode,
            config: config.clone(),
            workspace,
            web_search_enabled: config.enable_free_web_search,
            pending_confirmations: Vec::new(),
            input_history: Vec::new(),
        }
    }

    pub fn switch_mode(&mut self) -> AppMode {
        self.mode = self.mode.cycle();
        self.mode
    }

    pub fn push_pending(&mut self, action: PendingAction) {
        self.pending_confirmations.push(action);
    }

    pub fn pop_pending(&mut self) -> Option<PendingAction> {
        self.pending_confirmations.pop()
    }

    pub fn clear_pending(&mut self) {
        self.pending_confirmations.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> AppState {
        AppState::new(Config::default(), PathBuf::from("/tmp/test"))
    }

    #[test]
    fn mode_cycle() {
        assert_eq!(AppMode::Plan.cycle(), AppMode::Edit);
        assert_eq!(AppMode::Edit.cycle(), AppMode::Auto);
        assert_eq!(AppMode::Auto.cycle(), AppMode::Plan);
    }

    #[test]
    fn plan_permissions() {
        let mode = AppMode::Plan;
        assert!(!mode.can_write_file());
        assert!(!mode.can_execute_command());
        assert!(!mode.needs_confirmation());
    }

    #[test]
    fn edit_permissions() {
        let mode = AppMode::Edit;
        assert!(mode.can_write_file());
        assert!(mode.can_execute_command());
        assert!(mode.needs_confirmation());
    }

    #[test]
    fn auto_permissions() {
        let mode = AppMode::Auto;
        assert!(mode.can_write_file());
        assert!(mode.can_execute_command());
        assert!(!mode.needs_confirmation());
    }

    #[test]
    fn state_switch_mode() {
        let mut state = test_state();
        assert_eq!(state.mode, AppMode::Edit);
        state.switch_mode();
        assert_eq!(state.mode, AppMode::Auto);
        state.switch_mode();
        assert_eq!(state.mode, AppMode::Plan);
    }

    #[test]
    fn pending_actions() {
        let mut state = test_state();
        state.push_pending(PendingAction::ExecuteCommand {
            cmd: "cargo".into(),
            args: vec!["build".into()],
        });
        assert_eq!(state.pending_confirmations.len(), 1);
        let popped = state.pop_pending();
        assert!(popped.is_some());
        assert!(state.pending_confirmations.is_empty());
    }

    #[test]
    fn from_str_lossy() {
        assert_eq!(AppMode::from_str_lossy("plan"), AppMode::Plan);
        assert_eq!(AppMode::from_str_lossy("edit"), AppMode::Edit);
        assert_eq!(AppMode::from_str_lossy("auto"), AppMode::Auto);
        assert_eq!(AppMode::from_str_lossy("unknown"), AppMode::Edit);
    }
}
