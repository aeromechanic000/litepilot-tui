use crate::config::ThemeColors;
use ratatui::style::Color;

pub struct Theme {
    pub primary: Color,
    pub accent: Color,
    pub warning: Color,
}

impl Theme {
    pub fn from_config(colors: &ThemeColors) -> Self {
        Self {
            primary: parse_color(&colors.primary).unwrap_or(Color::Cyan),
            accent: parse_color(&colors.accent).unwrap_or(Color::Magenta),
            warning: parse_color(&colors.warning).unwrap_or(Color::Yellow),
        }
    }

    pub fn mode_indicator(&self, mode: &crate::app::AppMode) -> (&'static str, Color) {
        match mode {
            crate::app::AppMode::Plan => ("PLAN", self.primary),
            crate::app::AppMode::Edit => ("EDIT", self.primary),
            crate::app::AppMode::Auto => ("AUTO", self.primary),
        }
    }
}

fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    match s.to_ascii_lowercase().as_str() {
        // Standard ANSI
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

        // Reds
        "crimson" => Some(rgb(220, 20, 60)),
        "firebrick" => Some(rgb(178, 34, 34)),
        "darkred" => Some(rgb(139, 0, 0)),
        "indianred" => Some(rgb(205, 92, 92)),
        "orangered" => Some(rgb(255, 69, 0)),
        "tomato" => Some(rgb(255, 99, 71)),
        "coral" => Some(rgb(255, 127, 80)),
        "salmon" => Some(rgb(250, 128, 114)),
        "darksalmon" => Some(rgb(233, 150, 122)),
        "rosybrown" => Some(rgb(188, 143, 143)),

        // Greens
        "lime" => Some(rgb(0, 255, 0)),
        "forestgreen" => Some(rgb(34, 139, 34)),
        "seagreen" => Some(rgb(46, 139, 87)),
        "springgreen" => Some(rgb(0, 255, 127)),
        "limegreen" => Some(rgb(50, 205, 50)),
        "palegreen" => Some(rgb(152, 251, 152)),
        "darkgreen" => Some(rgb(0, 100, 0)),
        "olive" => Some(rgb(128, 128, 0)),
        "olivedrab" => Some(rgb(107, 142, 35)),
        "chartreuse" => Some(rgb(127, 255, 0)),

        // Blues
        "navy" => Some(rgb(0, 0, 128)),
        "steelblue" => Some(rgb(70, 130, 180)),
        "cornflowerblue" => Some(rgb(100, 149, 237)),
        "dodgerblue" => Some(rgb(30, 144, 255)),
        "royalblue" => Some(rgb(65, 105, 225)),
        "deepskyblue" => Some(rgb(0, 191, 255)),
        "mediumblue" => Some(rgb(0, 0, 205)),
        "slateblue" => Some(rgb(106, 90, 205)),
        "skyblue" => Some(rgb(135, 206, 235)),
        "powderblue" => Some(rgb(176, 224, 230)),

        // Yellows / Oranges
        "gold" => Some(rgb(255, 215, 0)),
        "orange" => Some(rgb(255, 165, 0)),
        "darkorange" => Some(rgb(255, 140, 0)),
        "goldenrod" => Some(rgb(218, 165, 32)),
        "darkgoldenrod" => Some(rgb(184, 134, 11)),
        "khaki" => Some(rgb(240, 230, 140)),
        "amber" => Some(rgb(255, 191, 0)),
        "peach" => Some(rgb(255, 218, 185)),
        "apricot" => Some(rgb(251, 206, 177)),
        "wheat" => Some(rgb(245, 222, 179)),

        // Purples / Pinks
        "violet" => Some(rgb(238, 130, 238)),
        "plum" => Some(rgb(221, 160, 221)),
        "orchid" => Some(rgb(218, 112, 214)),
        "purple" => Some(rgb(128, 0, 128)),
        "hotpink" => Some(rgb(255, 105, 180)),
        "deeppink" => Some(rgb(255, 20, 147)),
        "mediumvioletred" => Some(rgb(199, 21, 133)),
        "palevioletred" => Some(rgb(219, 112, 147)),
        "fuchsia" => Some(rgb(255, 0, 255)),
        "lavender" => Some(rgb(230, 230, 250)),

        // Cyans / Teals
        "teal" => Some(rgb(0, 128, 128)),
        "turquoise" => Some(rgb(64, 224, 208)),
        "aquamarine" => Some(rgb(127, 255, 212)),
        "darkcyan" => Some(rgb(0, 139, 139)),
        "cadetblue" => Some(rgb(95, 158, 160)),
        "mediumturquoise" => Some(rgb(72, 209, 204)),
        "paleturquoise" => Some(rgb(175, 238, 238)),
        "lightseagreen" => Some(rgb(32, 178, 170)),
        "darkturquoise" => Some(rgb(0, 206, 209)),
        "mediumaquamarine" => Some(rgb(102, 205, 170)),

        // Neutrals
        "silver" => Some(rgb(192, 192, 192)),
        "slategray" => Some(rgb(112, 128, 144)),
        "lightslategray" => Some(rgb(119, 136, 153)),
        "dimgray" => Some(rgb(105, 105, 105)),
        "gainsboro" => Some(rgb(220, 220, 220)),
        "whitesmoke" => Some(rgb(245, 245, 245)),
        "midnightblue" => Some(rgb(25, 25, 112)),
        "snow" => Some(rgb(255, 250, 250)),
        "ivory" => Some(rgb(255, 255, 240)),
        "seashell" => Some(rgb(255, 245, 238)),

        _ => parse_hex(s),
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
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
        assert_eq!(theme.primary, Color::Cyan);
        assert_eq!(theme.accent, Color::Magenta);
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
        assert_eq!(theme.primary, Color::Cyan);
    }
}
