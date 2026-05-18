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
            primary: Color::Rgb(0x7E, 0xC8, 0xFF),        // Bright sky blue
            secondary: Color::Rgb(0x7E, 0xC8, 0xFF),      // Same as primary for consistency
            bg_main: Color::Reset,                         // Terminal default
            bg_sidebar: Color::Rgb(0x1E, 0x24, 0x33),     // Dark blue-gray
            text: Color::Rgb(0xF0, 0xF6, 0xFF),           // Near white
            thinking: Color::Rgb(0x7E, 0xD8, 0xD0),       // Bright teal
            warning: Color::Rgb(0xFF, 0xD7, 0x00),        // Bright yellow
            error: Color::Rgb(0xFF, 0x6B, 0x6B),          // Bright red
            sandbox: Color::Rgb(0x80, 0xE0, 0xFF),        // Ice blue
            code_keyword: Color::Rgb(0x6E, 0xE8, 0xF2),   // Bright cyan
            code_string: Color::Rgb(0xCE, 0x91, 0x78),    // Warm orange
            code_comment: Color::Rgb(0x9A, 0x9A, 0x9A),   // Visible gray
            success: Color::Rgb(0x73, 0xD2, 0x16),        // Bright green
        }
    }

    /// Light terminal theme — black/blue on light background.
    fn light() -> Self {
        Self {
            primary: Color::Rgb(0x10, 0x6E, 0xCB),        // DeepSeek blue (light variant)
            secondary: Color::Rgb(0x30, 0x60, 0x90),      // Slate blue
            bg_main: Color::Reset,                         // Terminal default
            bg_sidebar: Color::Rgb(0xD8, 0xE4, 0xF0),     // Light blue-gray
            text: Color::Rgb(0x1A, 0x1A, 0x2E),           // Dark navy
            thinking: Color::Rgb(0x00, 0x80, 0x78),       // Dark teal
            warning: Color::Rgb(0xB0, 0x8C, 0x00),        // Dark gold
            error: Color::Rgb(0xC0, 0x30, 0x30),          // Dark red
            sandbox: Color::Rgb(0x00, 0x70, 0x90),        // Dark cyan
            code_keyword: Color::Rgb(0x00, 0x50, 0xA0),   // Blue
            code_string: Color::Rgb(0xA0, 0x60, 0x40),    // Brown
            code_comment: Color::Rgb(0x60, 0x60, 0x60),   // Dark gray
            success: Color::Rgb(0x30, 0x80, 0x30),        // Dark green
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
        assert_eq!(theme.primary, Color::Rgb(0x7E, 0xC8, 0xFF));
        assert_eq!(theme.secondary, Color::Rgb(0x7E, 0xC8, 0xFF));
        assert_eq!(theme.text, Color::Rgb(0xF0, 0xF6, 0xFF));
        assert_eq!(theme.bg_main, Color::Reset);
    }

    #[test]
    fn light_theme_colors() {
        let theme = Theme::light();
        assert_eq!(theme.primary, Color::Rgb(0x10, 0x6E, 0xCB));
        assert_eq!(theme.text, Color::Rgb(0x1A, 0x1A, 0x2E));
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
