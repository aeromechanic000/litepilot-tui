use ratatui::style::Color;
use crate::config::ThemeColors;

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
    /// Build theme from config colors, falling back to defaults for unparseable values.
    pub fn from_config(colors: &ThemeColors) -> Self {
        Self {
            primary: parse_color(&colors.primary).unwrap_or(Color::Rgb(0x67, 0xE8, 0xF9)),
            secondary: parse_color(&colors.secondary).unwrap_or(Color::Rgb(0xA5, 0xF3, 0xFC)),
            bg_main: parse_color(&colors.bg_main).unwrap_or(Color::Reset),
            bg_sidebar: parse_color(&colors.bg_sidebar).unwrap_or(Color::Rgb(0x16, 0x4E, 0x63)),
            text: parse_color(&colors.text).unwrap_or(Color::Rgb(0xE0, 0xF7, 0xFA)),
            thinking: parse_color(&colors.thinking).unwrap_or(Color::Rgb(0x5E, 0xEA, 0xD4)),
            warning: parse_color(&colors.warning).unwrap_or(Color::Rgb(0xFD, 0xE6, 0x8A)),
            error: parse_color(&colors.error).unwrap_or(Color::Rgb(0xFC, 0xA5, 0xA5)),
            sandbox: parse_color(&colors.sandbox).unwrap_or(Color::Rgb(0x67, 0xE8, 0xF9)),
            code_keyword: parse_color(&colors.code_keyword).unwrap_or(Color::Rgb(0x67, 0xE8, 0xF9)),
            code_string: parse_color(&colors.code_string).unwrap_or(Color::Rgb(0xA5, 0xF3, 0xFC)),
            code_comment: parse_color(&colors.code_comment).unwrap_or(Color::Rgb(0xB0, 0xBE, 0xC5)),
            success: parse_color(&colors.success).unwrap_or(Color::Rgb(0x6E, 0xE7, 0xB7)),
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

/// Parse a hex color string like "#06B6D4" or the special value "reset".
/// Returns None for unparseable strings.
fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("reset") {
        return Some(Color::Reset);
    }
    let hex = s.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_config(&ThemeColors::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_uses_config_colors() {
        let theme = Theme::default();
        assert_eq!(theme.primary, parse_color(&ThemeColors::default().primary).unwrap());
        assert_eq!(theme.bg_main, Color::Reset);
    }

    #[test]
    fn mode_indicators() {
        let theme = Theme::default();
        let (label, _) = theme.mode_indicator(&crate::app::AppMode::Plan);
        assert_eq!(label, "PLAN");
        let (label, _) = theme.mode_indicator(&crate::app::AppMode::Edit);
        assert_eq!(label, "EDIT");
        let (label, _) = theme.mode_indicator(&crate::app::AppMode::Auto);
        assert_eq!(label, "AUTO");
    }

    #[test]
    fn parse_color_hex() {
        assert_eq!(parse_color("#06B6D4"), Some(Color::Rgb(0x06, 0xB6, 0xD4)));
        assert_eq!(parse_color("#000000"), Some(Color::Rgb(0, 0, 0)));
        assert_eq!(parse_color("#FFFFFF"), Some(Color::Rgb(0xFF, 0xFF, 0xFF)));
    }

    #[test]
    fn parse_color_reset() {
        assert_eq!(parse_color("reset"), Some(Color::Reset));
        assert_eq!(parse_color("Reset"), Some(Color::Reset));
        assert_eq!(parse_color(" RESET "), Some(Color::Reset));
    }

    #[test]
    fn parse_color_invalid() {
        assert_eq!(parse_color(""), None);
        assert_eq!(parse_color("#FFF"), None);
        assert_eq!(parse_color("notacolor"), None);
        assert_eq!(parse_color("#GGGGGG"), None);
    }

    #[test]
    fn from_config_uses_provided_colors() {
        let colors = ThemeColors {
            primary: "#FF0000".into(),
            ..ThemeColors::default()
        };
        let theme = Theme::from_config(&colors);
        assert_eq!(theme.primary, Color::Rgb(0xFF, 0, 0));
        assert_eq!(theme.bg_main, Color::Reset);
    }

    #[test]
    fn from_config_handles_invalid_colors() {
        let colors = ThemeColors {
            primary: "not-a-color".into(),
            ..ThemeColors::default()
        };
        let theme = Theme::from_config(&colors);
        assert_eq!(theme.primary, Color::Rgb(0x67, 0xE8, 0xF9)); // hardcoded fallback
    }
}
