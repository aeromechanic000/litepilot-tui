use ratatui::style::Color;
use crate::config::ThemeColors;

pub struct Theme {
    pub primary: Color,
    pub accent: Color,
    pub warning: Color,
}

impl Theme {
    pub fn from_config(colors: &ThemeColors) -> Self {
        Self {
            primary: parse_color(&colors.primary).unwrap_or(Color::Blue),
            accent: parse_color(&colors.accent).unwrap_or(Color::Cyan),
            warning: parse_color(&colors.warning).unwrap_or(Color::Yellow),
        }
    }

    pub fn mode_indicator(&self, mode: &crate::app::AppMode) -> (&'static str, Color) {
        match mode {
            crate::app::AppMode::Plan => ("PLAN", self.accent),
            crate::app::AppMode::Edit => ("EDIT", self.primary),
            crate::app::AppMode::Auto => ("AUTO", self.warning),
        }
    }
}

fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    match s.to_ascii_lowercase().as_str() {
        "reset" => Some(Color::Reset),
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "darkgray" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        _ => parse_hex(s),
    }
}

fn parse_hex(s: &str) -> Option<Color> {
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
        assert_eq!(theme.primary, Color::Blue);
        assert_eq!(theme.accent, Color::Cyan);
        assert_eq!(theme.warning, Color::Yellow);
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
    fn parse_color_ansi_names() {
        assert_eq!(parse_color("blue"), Some(Color::Blue));
        assert_eq!(parse_color("Blue"), Some(Color::Blue));
        assert_eq!(parse_color("BLUE"), Some(Color::Blue));
        assert_eq!(parse_color("cyan"), Some(Color::Cyan));
        assert_eq!(parse_color("yellow"), Some(Color::Yellow));
        assert_eq!(parse_color("red"), Some(Color::Red));
        assert_eq!(parse_color("green"), Some(Color::Green));
        assert_eq!(parse_color("magenta"), Some(Color::Magenta));
        assert_eq!(parse_color("white"), Some(Color::White));
        assert_eq!(parse_color("black"), Some(Color::Black));
        assert_eq!(parse_color("reset"), Some(Color::Reset));
        assert_eq!(parse_color("gray"), Some(Color::DarkGray));
        assert_eq!(parse_color("darkgray"), Some(Color::DarkGray));
        assert_eq!(parse_color("lightred"), Some(Color::LightRed));
        assert_eq!(parse_color("lightyellow"), Some(Color::LightYellow));
        assert_eq!(parse_color("lightcyan"), Some(Color::LightCyan));
    }

    #[test]
    fn parse_color_hex() {
        assert_eq!(parse_color("#315DFC"), Some(Color::Rgb(0x31, 0x5D, 0xFC)));
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
    fn from_config_uses_ansi_names() {
        let colors = ThemeColors {
            primary: "red".into(),
            ..ThemeColors::default()
        };
        let theme = Theme::from_config(&colors);
        assert_eq!(theme.primary, Color::Red);
    }

    #[test]
    fn from_config_uses_hex() {
        let colors = ThemeColors {
            primary: "#FF0000".into(),
            ..ThemeColors::default()
        };
        let theme = Theme::from_config(&colors);
        assert_eq!(theme.primary, Color::Rgb(0xFF, 0, 0));
    }

    #[test]
    fn from_config_handles_invalid_colors() {
        let colors = ThemeColors {
            primary: "not-a-color".into(),
            ..ThemeColors::default()
        };
        let theme = Theme::from_config(&colors);
        assert_eq!(theme.primary, Color::Blue);
    }
}
