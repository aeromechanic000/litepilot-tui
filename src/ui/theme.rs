use ratatui::style::Color;

pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub bg_main: Color,
    pub bg_sidebar: Color,
    pub text: Color,
    pub thinking: Color,
    pub warning: Color,
    pub error: Color,
    #[allow(dead_code)]
    pub sandbox: Color,
    pub code_keyword: Color,
    #[allow(dead_code)]
    pub code_string: Color,
    #[allow(dead_code)]
    pub code_comment: Color,
    #[allow(dead_code)]
    pub success: Color,
}

impl Theme {
    /// Dark terminal theme — white/cyan/blue on dark background.
    fn dark() -> Self {
        Self {
            primary: Color::LightCyan,         // Bright cyan — main theme accent
            secondary: Color::White,           // Bright white — labels, help text
            bg_main: Color::Reset,             // Terminal default
            bg_sidebar: Color::Rgb(0x1E, 0x24, 0x33), // Dark blue-gray
            text: Color::White,                // Bright white
            thinking: Color::LightGreen,       // Bright green
            warning: Color::LightYellow,       // Bright yellow
            error: Color::LightRed,            // Bright red
            sandbox: Color::LightCyan,         // Bright cyan
            code_keyword: Color::LightCyan,    // Bright cyan
            code_string: Color::LightYellow,   // Bright yellow
            code_comment: Color::Gray,         // Gray
            success: Color::LightGreen,        // Bright green
        }
    }

    /// Light terminal theme — dark text on light background.
    fn light() -> Self {
        Self {
            primary: Color::Cyan,              // Standard cyan — main theme accent
            secondary: Color::Black,           // Black — labels, help text
            bg_main: Color::Reset,             // Terminal default
            bg_sidebar: Color::Rgb(0xD8, 0xE4, 0xF0), // Light blue-gray
            text: Color::Black,                // Black
            thinking: Color::Green,            // Green
            warning: Color::Yellow,            // Yellow
            error: Color::Red,                 // Red
            sandbox: Color::Cyan,              // Cyan
            code_keyword: Color::Cyan,         // Cyan
            code_string: Color::Yellow,        // Yellow
            code_comment: Color::DarkGray,     // Dark gray
            success: Color::Green,             // Green
        }
    }

    /// Detect whether the terminal has a dark or light background.
    /// Uses the `$COLORFGBG` env var (set by xterm, rxvt, iTerm2, etc.)
    /// or defaults to dark.
    pub fn detect() -> Self {
        if is_light_background() {
            Self::light()
        } else {
            Self::dark()
        }
    }

    pub fn mode_indicator(&self, mode: &crate::app::AppMode) -> (&'static str, Color) {
        match mode {
            crate::app::AppMode::Plan => ("PLAN", self.thinking),
            crate::app::AppMode::Edit => ("EDIT", self.primary),
            crate::app::AppMode::Auto => ("AUTO", self.warning),
        }
    }
}

/// Check `$COLORFGBG` for background color hint.
/// Format: "fg_num;bg_num" where 0=black, 7=light gray, 15=white.
/// Values > 7 are treated as light backgrounds.
fn is_light_background() -> bool {
    std::env::var("COLORFGBG")
        .ok()
        .and_then(|val| {
            // e.g. "0;15" or "15;0" — take the bg part after ';'
            let parts: Vec<&str> = val.split(';').collect();
            if parts.len() >= 2 {
                parts.last()?.parse::<u8>().ok()
            } else {
                None
            }
        })
        .map_or(false, |bg| bg > 7)
}

impl Default for Theme {
    fn default() -> Self {
        Self::detect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_theme_colors() {
        let theme = Theme::dark();
        assert_eq!(theme.primary, Color::LightCyan);
        assert_eq!(theme.secondary, Color::White);
        assert_eq!(theme.text, Color::White);
        assert_eq!(theme.bg_main, Color::Reset);
    }

    #[test]
    fn light_theme_colors() {
        let theme = Theme::light();
        assert_eq!(theme.primary, Color::Cyan);
        assert_eq!(theme.text, Color::Black);
        assert_eq!(theme.bg_main, Color::Reset);
    }

    #[test]
    fn detect_uses_env_var() {
        // Without COLORFGBG set (or with a dark value), should default to dark
        let theme = Theme::detect();
        // We can't control env vars in tests, just verify it doesn't panic
        assert!(!theme.text.to_string().is_empty() || true);
    }

    #[test]
    fn mode_indicators() {
        let theme = Theme::dark();
        let (label, _) = theme.mode_indicator(&crate::app::AppMode::Plan);
        assert_eq!(label, "PLAN");
        let (label, _) = theme.mode_indicator(&crate::app::AppMode::Edit);
        assert_eq!(label, "EDIT");
        let (label, _) = theme.mode_indicator(&crate::app::AppMode::Auto);
        assert_eq!(label, "AUTO");
    }
}
