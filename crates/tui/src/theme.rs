use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreset {
    NyzhiDark,
    NyzhiLight,
    TokyoNight,
    CatppuccinMocha,
    Dracula,
    SolarizedDark,
    SolarizedLight,
    GruvboxDark,
}

impl ThemePreset {
    pub const ALL: &[ThemePreset] = &[
        ThemePreset::NyzhiDark,
        ThemePreset::NyzhiLight,
        ThemePreset::TokyoNight,
        ThemePreset::CatppuccinMocha,
        ThemePreset::Dracula,
        ThemePreset::SolarizedDark,
        ThemePreset::SolarizedLight,
        ThemePreset::GruvboxDark,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ThemePreset::NyzhiDark => "nyzhi-dark",
            ThemePreset::NyzhiLight => "nyzhi-light",
            ThemePreset::TokyoNight => "tokyonight",
            ThemePreset::CatppuccinMocha => "catppuccin-mocha",
            ThemePreset::Dracula => "dracula",
            ThemePreset::SolarizedDark => "solarized-dark",
            ThemePreset::SolarizedLight => "solarized-light",
            ThemePreset::GruvboxDark => "gruvbox-dark",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            ThemePreset::NyzhiDark => "Nyzhi Dark",
            ThemePreset::NyzhiLight => "Nyzhi Light",
            ThemePreset::TokyoNight => "Tokyo Night",
            ThemePreset::CatppuccinMocha => "Catppuccin Mocha",
            ThemePreset::Dracula => "Dracula",
            ThemePreset::SolarizedDark => "Solarized Dark",
            ThemePreset::SolarizedLight => "Solarized Light",
            ThemePreset::GruvboxDark => "Gruvbox Dark",
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().replace(' ', "-").as_str() {
            "dark" | "nyzhi-dark" | "nyzhidark" => ThemePreset::NyzhiDark,
            "light" | "nyzhi-light" | "nyzhi_light" | "nyzhilight" => ThemePreset::NyzhiLight,
            "tokyonight" | "tokyo-night" | "tokyo_night" => ThemePreset::TokyoNight,
            "catppuccin-mocha" | "catppuccin_mocha" | "catppuccinmocha" | "catppuccin" => {
                ThemePreset::CatppuccinMocha
            }
            "dracula" => ThemePreset::Dracula,
            "solarized-dark" | "solarized_dark" | "solarizeddark" => ThemePreset::SolarizedDark,
            "solarized-light" | "solarized_light" | "solarizedlight" => {
                ThemePreset::SolarizedLight
            }
            "gruvbox-dark" | "gruvbox_dark" | "gruvboxdark" | "gruvbox" => {
                ThemePreset::GruvboxDark
            }
            _ => ThemePreset::NyzhiDark,
        }
    }

    pub fn mode(self) -> ThemeMode {
        match self {
            ThemePreset::NyzhiLight | ThemePreset::SolarizedLight => ThemeMode::Light,
            _ => ThemeMode::Dark,
        }
    }

    pub fn bg_page_color(self) -> Color {
        match self {
            ThemePreset::NyzhiDark => Color::Rgb(0, 0, 0),
            ThemePreset::NyzhiLight => Color::Rgb(245, 240, 232),
            ThemePreset::TokyoNight => Color::Rgb(26, 27, 38),
            ThemePreset::CatppuccinMocha => Color::Rgb(30, 30, 46),
            ThemePreset::Dracula => Color::Rgb(40, 42, 54),
            ThemePreset::SolarizedDark => Color::Rgb(0, 43, 54),
            ThemePreset::SolarizedLight => Color::Rgb(253, 246, 227),
            ThemePreset::GruvboxDark => Color::Rgb(40, 40, 40),
        }
    }

    pub fn palette(self) -> Theme {
        let mode = self.mode();
        let accent = Accent::Copper;
        match self {
            ThemePreset::NyzhiDark => Theme {
                mode,
                preset: self,
                accent_type: accent,
                bg_page: Color::Rgb(0, 0, 0),
                bg_surface: Color::Rgb(0, 0, 0),
                bg_elevated: Color::Rgb(10, 10, 10),
                bg_sunken: Color::Rgb(0, 0, 0),
                text_primary: Color::Rgb(250, 250, 250),
                text_secondary: Color::Rgb(161, 161, 170),
                text_tertiary: Color::Rgb(113, 113, 122),
                text_disabled: Color::Rgb(63, 63, 70),
                border_default: Color::Rgb(26, 26, 26),
                border_strong: Color::Rgb(41, 41, 41),
                accent: accent.color(mode),
                accent_muted: accent.muted(mode),
                success: Color::Rgb(74, 222, 128),
                danger: Color::Rgb(248, 113, 113),
                warning: Color::Rgb(251, 191, 36),
                info: Color::Rgb(161, 161, 170),
            },
            ThemePreset::NyzhiLight => Theme {
                mode,
                preset: self,
                accent_type: accent,
                bg_page: Color::Rgb(245, 240, 232),
                bg_surface: Color::Rgb(250, 246, 240),
                bg_elevated: Color::Rgb(254, 251, 246),
                bg_sunken: Color::Rgb(237, 231, 220),
                text_primary: Color::Rgb(28, 25, 23),
                text_secondary: Color::Rgb(87, 83, 78),
                text_tertiary: Color::Rgb(135, 130, 124),
                text_disabled: Color::Rgb(214, 211, 209),
                border_default: Color::Rgb(221, 214, 202),
                border_strong: Color::Rgb(207, 198, 184),
                accent: accent.color(mode),
                accent_muted: accent.muted(mode),
                success: Color::Rgb(16, 185, 129),
                danger: Color::Rgb(239, 68, 68),
                warning: Color::Rgb(245, 158, 11),
                info: Color::Rgb(113, 113, 122),
            },
            ThemePreset::TokyoNight => Theme {
                mode,
                preset: self,
                accent_type: accent,
                bg_page: Color::Rgb(26, 27, 38),
                bg_surface: Color::Rgb(31, 35, 53),
                bg_elevated: Color::Rgb(36, 40, 59),
                bg_sunken: Color::Rgb(22, 22, 30),
                text_primary: Color::Rgb(192, 202, 245),
                text_secondary: Color::Rgb(169, 177, 214),
                text_tertiary: Color::Rgb(86, 95, 137),
                text_disabled: Color::Rgb(59, 66, 97),
                border_default: Color::Rgb(41, 46, 66),
                border_strong: Color::Rgb(59, 66, 97),
                accent: accent.color(mode),
                accent_muted: accent.muted(mode),
                success: Color::Rgb(158, 206, 106),
                danger: Color::Rgb(247, 118, 142),
                warning: Color::Rgb(224, 175, 104),
                info: Color::Rgb(122, 162, 247),
            },
            ThemePreset::CatppuccinMocha => Theme {
                mode,
                preset: self,
                accent_type: accent,
                bg_page: Color::Rgb(30, 30, 46),
                bg_surface: Color::Rgb(49, 50, 68),
                bg_elevated: Color::Rgb(69, 71, 90),
                bg_sunken: Color::Rgb(24, 24, 37),
                text_primary: Color::Rgb(205, 214, 244),
                text_secondary: Color::Rgb(186, 194, 222),
                text_tertiary: Color::Rgb(108, 112, 134),
                text_disabled: Color::Rgb(69, 71, 90),
                border_default: Color::Rgb(49, 50, 68),
                border_strong: Color::Rgb(69, 71, 90),
                accent: accent.color(mode),
                accent_muted: accent.muted(mode),
                success: Color::Rgb(166, 227, 161),
                danger: Color::Rgb(243, 139, 168),
                warning: Color::Rgb(249, 226, 175),
                info: Color::Rgb(137, 180, 250),
            },
            ThemePreset::Dracula => Theme {
                mode,
                preset: self,
                accent_type: accent,
                bg_page: Color::Rgb(40, 42, 54),
                bg_surface: Color::Rgb(52, 55, 70),
                bg_elevated: Color::Rgb(68, 71, 90),
                bg_sunken: Color::Rgb(33, 34, 44),
                text_primary: Color::Rgb(248, 248, 242),
                text_secondary: Color::Rgb(189, 192, 208),
                text_tertiary: Color::Rgb(98, 114, 164),
                text_disabled: Color::Rgb(68, 71, 90),
                border_default: Color::Rgb(68, 71, 90),
                border_strong: Color::Rgb(98, 114, 164),
                accent: accent.color(mode),
                accent_muted: accent.muted(mode),
                success: Color::Rgb(80, 250, 123),
                danger: Color::Rgb(255, 85, 85),
                warning: Color::Rgb(241, 250, 140),
                info: Color::Rgb(139, 233, 253),
            },
            ThemePreset::SolarizedDark => Theme {
                mode,
                preset: self,
                accent_type: accent,
                bg_page: Color::Rgb(0, 43, 54),
                bg_surface: Color::Rgb(7, 54, 66),
                bg_elevated: Color::Rgb(13, 74, 89),
                bg_sunken: Color::Rgb(0, 30, 38),
                text_primary: Color::Rgb(131, 148, 150),
                text_secondary: Color::Rgb(101, 123, 131),
                text_tertiary: Color::Rgb(88, 110, 117),
                text_disabled: Color::Rgb(46, 79, 88),
                border_default: Color::Rgb(7, 54, 66),
                border_strong: Color::Rgb(88, 110, 117),
                accent: accent.color(mode),
                accent_muted: accent.muted(mode),
                success: Color::Rgb(133, 153, 0),
                danger: Color::Rgb(220, 50, 47),
                warning: Color::Rgb(181, 137, 0),
                info: Color::Rgb(38, 139, 210),
            },
            ThemePreset::SolarizedLight => Theme {
                mode,
                preset: self,
                accent_type: accent,
                bg_page: Color::Rgb(253, 246, 227),
                bg_surface: Color::Rgb(255, 249, 237),
                bg_elevated: Color::Rgb(255, 252, 243),
                bg_sunken: Color::Rgb(238, 232, 213),
                text_primary: Color::Rgb(101, 123, 131),
                text_secondary: Color::Rgb(131, 148, 150),
                text_tertiary: Color::Rgb(147, 161, 161),
                text_disabled: Color::Rgb(211, 203, 183),
                border_default: Color::Rgb(238, 232, 213),
                border_strong: Color::Rgb(147, 161, 161),
                accent: accent.color(mode),
                accent_muted: accent.muted(mode),
                success: Color::Rgb(133, 153, 0),
                danger: Color::Rgb(220, 50, 47),
                warning: Color::Rgb(181, 137, 0),
                info: Color::Rgb(38, 139, 210),
            },
            ThemePreset::GruvboxDark => Theme {
                mode,
                preset: self,
                accent_type: accent,
                bg_page: Color::Rgb(40, 40, 40),
                bg_surface: Color::Rgb(60, 56, 54),
                bg_elevated: Color::Rgb(80, 73, 69),
                bg_sunken: Color::Rgb(29, 32, 33),
                text_primary: Color::Rgb(235, 219, 178),
                text_secondary: Color::Rgb(213, 196, 161),
                text_tertiary: Color::Rgb(168, 153, 132),
                text_disabled: Color::Rgb(102, 92, 84),
                border_default: Color::Rgb(60, 56, 54),
                border_strong: Color::Rgb(80, 73, 69),
                accent: accent.color(mode),
                accent_muted: accent.muted(mode),
                success: Color::Rgb(184, 187, 38),
                danger: Color::Rgb(251, 73, 52),
                warning: Color::Rgb(250, 189, 47),
                info: Color::Rgb(131, 165, 152),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accent {
    Copper,
    Blue,
    Orange,
    Emerald,
    Violet,
    Rose,
    Amber,
    Cyan,
    Red,
    Pink,
    Teal,
    Indigo,
    Lime,
    Monochrome,
}

impl Accent {
    pub const ALL: &[Accent] = &[
        Accent::Copper,
        Accent::Blue,
        Accent::Orange,
        Accent::Emerald,
        Accent::Violet,
        Accent::Rose,
        Accent::Amber,
        Accent::Cyan,
        Accent::Red,
        Accent::Pink,
        Accent::Teal,
        Accent::Indigo,
        Accent::Lime,
        Accent::Monochrome,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Accent::Copper => "copper",
            Accent::Blue => "blue",
            Accent::Orange => "orange",
            Accent::Emerald => "emerald",
            Accent::Violet => "violet",
            Accent::Rose => "rose",
            Accent::Amber => "amber",
            Accent::Cyan => "cyan",
            Accent::Red => "red",
            Accent::Pink => "pink",
            Accent::Teal => "teal",
            Accent::Indigo => "indigo",
            Accent::Lime => "lime",
            Accent::Monochrome => "monochrome",
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "blue" => Accent::Blue,
            "orange" => Accent::Orange,
            "emerald" => Accent::Emerald,
            "violet" => Accent::Violet,
            "rose" => Accent::Rose,
            "amber" => Accent::Amber,
            "cyan" => Accent::Cyan,
            "red" => Accent::Red,
            "pink" => Accent::Pink,
            "teal" => Accent::Teal,
            "indigo" => Accent::Indigo,
            "lime" => Accent::Lime,
            "monochrome" => Accent::Monochrome,
            _ => Accent::Copper,
        }
    }

    pub fn next(self) -> Self {
        let idx = Accent::ALL.iter().position(|&a| a == self).unwrap_or(0);
        Accent::ALL[(idx + 1) % Accent::ALL.len()]
    }

    pub fn color_preview(self, mode: ThemeMode) -> Color {
        self.color(mode)
    }

    fn color(self, mode: ThemeMode) -> Color {
        match self {
            Accent::Copper => Color::Rgb(196, 154, 108),
            Accent::Blue => Color::Rgb(59, 130, 246),
            Accent::Orange => Color::Rgb(255, 102, 0),
            Accent::Emerald => Color::Rgb(16, 185, 129),
            Accent::Violet => Color::Rgb(139, 92, 246),
            Accent::Rose => Color::Rgb(244, 63, 94),
            Accent::Amber => Color::Rgb(245, 158, 11),
            Accent::Cyan => Color::Rgb(6, 182, 212),
            Accent::Red => Color::Rgb(239, 68, 68),
            Accent::Pink => Color::Rgb(236, 72, 153),
            Accent::Teal => Color::Rgb(20, 184, 166),
            Accent::Indigo => Color::Rgb(99, 102, 241),
            Accent::Lime => Color::Rgb(132, 204, 22),
            Accent::Monochrome => match mode {
                ThemeMode::Light => Color::Rgb(130, 91, 50),
                ThemeMode::Dark => Color::Rgb(196, 154, 108),
            },
        }
    }

    fn muted(self, mode: ThemeMode) -> Color {
        let base = self.color(mode);
        let (br, bg, bb) = match mode {
            ThemeMode::Dark => (0u8, 0u8, 0u8),
            ThemeMode::Light => (245u8, 240u8, 232u8),
        };
        if let Color::Rgb(r, g, b) = base {
            Color::Rgb(blend(r, br, 38), blend(g, bg, 38), blend(b, bb, 38))
        } else {
            base
        }
    }
}

fn blend(fg: u8, bg: u8, alpha: u8) -> u8 {
    let a = alpha as u16;
    ((fg as u16 * a + bg as u16 * (255 - a)) / 255) as u8
}

fn parse_hex_color(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub mode: ThemeMode,
    pub preset: ThemePreset,
    pub accent_type: Accent,

    pub bg_page: Color,
    pub bg_surface: Color,
    pub bg_elevated: Color,
    pub bg_sunken: Color,

    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_tertiary: Color,
    pub text_disabled: Color,

    pub border_default: Color,
    pub border_strong: Color,

    pub accent: Color,
    pub accent_muted: Color,

    pub success: Color,
    pub danger: Color,
    pub warning: Color,
    pub info: Color,
}

impl Theme {
    pub fn new(preset: ThemePreset, accent: Accent) -> Self {
        let mut theme = preset.palette();
        theme.accent_type = accent;
        theme.accent = accent.color(theme.mode);
        theme.accent_muted = accent.muted(theme.mode);
        theme
    }

    pub fn from_config(config: &nyzhi_config::TuiConfig) -> Self {
        let preset = ThemePreset::from_name(&config.theme);
        let accent = Accent::from_name(&config.accent);
        let mut theme = Self::new(preset, accent);
        theme.apply_overrides(&config.colors);
        theme
    }

    pub fn apply_overrides(&mut self, overrides: &nyzhi_config::ThemeOverrides) {
        macro_rules! apply {
            ($field:ident) => {
                if let Some(ref hex) = overrides.$field {
                    if let Some(c) = parse_hex_color(hex) {
                        self.$field = c;
                    }
                }
            };
        }
        apply!(bg_page);
        apply!(bg_surface);
        apply!(bg_elevated);
        apply!(bg_sunken);
        apply!(text_primary);
        apply!(text_secondary);
        apply!(text_tertiary);
        apply!(text_disabled);
        apply!(border_default);
        apply!(border_strong);
        apply!(accent);
        apply!(accent_muted);
        apply!(success);
        apply!(danger);
        apply!(warning);
        apply!(info);
    }

    pub fn next_preset(&mut self) {
        let idx = ThemePreset::ALL
            .iter()
            .position(|&p| p == self.preset)
            .unwrap_or(0);
        let next = ThemePreset::ALL[(idx + 1) % ThemePreset::ALL.len()];
        *self = Self::new(next, self.accent_type);
    }

    pub fn next_accent(&mut self) {
        let next = self.accent_type.next();
        *self = Self::new(self.preset, next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_round_trip() {
        for preset in ThemePreset::ALL {
            let name = preset.name();
            let parsed = ThemePreset::from_name(name);
            assert_eq!(*preset, parsed, "round-trip failed for {name}");
        }
    }

    #[test]
    fn backward_compat_dark_light() {
        assert_eq!(ThemePreset::from_name("dark"), ThemePreset::NyzhiDark);
        assert_eq!(ThemePreset::from_name("light"), ThemePreset::NyzhiLight);
    }

    #[test]
    fn all_presets_produce_valid_theme() {
        for preset in ThemePreset::ALL {
            let theme = Theme::new(*preset, Accent::Blue);
            assert_eq!(theme.preset, *preset);
            assert_eq!(theme.accent_type, Accent::Blue);
        }
    }

    #[test]
    fn parse_hex() {
        assert_eq!(parse_hex_color("#ff0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_hex_color("00ff00"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(parse_hex_color("#xyz"), None);
        assert_eq!(parse_hex_color(""), None);
    }

    #[test]
    fn overrides_applied() {
        let mut theme = Theme::new(ThemePreset::NyzhiDark, Accent::Copper);
        let overrides = nyzhi_config::ThemeOverrides {
            bg_page: Some("#ff0000".to_string()),
            text_primary: Some("#00ff00".to_string()),
            ..Default::default()
        };
        theme.apply_overrides(&overrides);
        assert_eq!(theme.bg_page, Color::Rgb(255, 0, 0));
        assert_eq!(theme.text_primary, Color::Rgb(0, 255, 0));
    }
}
