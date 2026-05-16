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
    pub sandbox: Color,
    pub code_keyword: Color,
    pub code_string: Color,
    pub code_comment: Color,
    pub success: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: Color::Rgb(0x16, 0x5D, 0xFF),       // DeepSpaceBlue #165DFF
            secondary: Color::Rgb(0x40, 0x80, 0xFF),     // FogBlue #4080FF
            bg_main: Color::Rgb(0x1E, 0x22, 0x28),       // CharcoalGray #1E2228
            bg_sidebar: Color::Rgb(0x23, 0x27, 0x33),    // DarkBlueGray #232733
            text: Color::Rgb(0xCC, 0xCC, 0xCC),          // LightGrayWhite
            thinking: Color::Rgb(0x80, 0xCB, 0xC4),      // LightCyanBlue
            warning: Color::Rgb(0xFF, 0xD7, 0x00),       // LightYellow
            error: Color::Rgb(0xFF, 0x6B, 0x6B),         // LightRed
            sandbox: Color::Rgb(0x80, 0xE0, 0xFF),       // IceBlue
            code_keyword: Color::Rgb(0x56, 0x9C, 0xD6),  // Blue keyword
            code_string: Color::Rgb(0xCE, 0x91, 0x78),   // Warm string
            code_comment: Color::Rgb(0x6A, 0x6A, 0x6A),  // Dim gray
            success: Color::Rgb(0x73, 0xD2, 0x16),       // Green
        }
    }
}

impl Theme {
    pub fn mode_indicator(&self, mode: &crate::app::AppMode) -> (&'static str, Color) {
        match mode {
            crate::app::AppMode::Plan => ("PLAN", self.thinking),
            crate::app::AppMode::Edit => ("EDIT", self.primary),
            crate::app::AppMode::Auto => ("AUTO", self.warning),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_colors() {
        let theme = Theme::default();
        assert_eq!(theme.primary, Color::Rgb(0x16, 0x5D, 0xFF));
        assert_eq!(theme.bg_main, Color::Rgb(0x1E, 0x22, 0x28));
    }

    #[test]
    fn mode_indicators() {
        let theme = Theme::default();
        let (label, _color) = theme.mode_indicator(&crate::app::AppMode::Plan);
        assert_eq!(label, "PLAN");
        let (label, _) = theme.mode_indicator(&crate::app::AppMode::Edit);
        assert_eq!(label, "EDIT");
        let (label, _) = theme.mode_indicator(&crate::app::AppMode::Auto);
        assert_eq!(label, "AUTO");
    }
}
