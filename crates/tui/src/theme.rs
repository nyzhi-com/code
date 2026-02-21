use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
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
        // ~15% opacity approximation blended on the background
        let base = self.color(mode);
        let (br, bg, bb) = match mode {
            ThemeMode::Dark => (0u8, 0u8, 0u8),
            ThemeMode::Light => (245u8, 240u8, 232u8),
        };
        if let Color::Rgb(r, g, b) = base {
            Color::Rgb(
                blend(r, br, 38), // 15% of 255 ≈ 38
                blend(g, bg, 38),
                blend(b, bb, 38),
            )
        } else {
            base
        }
    }
}

fn blend(fg: u8, bg: u8, alpha: u8) -> u8 {
    let a = alpha as u16;
    ((fg as u16 * a + bg as u16 * (255 - a)) / 255) as u8
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub mode: ThemeMode,
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
    pub fn new(mode: ThemeMode, accent: Accent) -> Self {
        let (bg_page, bg_surface, bg_elevated, bg_sunken) = match mode {
            ThemeMode::Dark => (
                Color::Rgb(0, 0, 0),
                Color::Rgb(0, 0, 0),
                Color::Rgb(10, 10, 10),
                Color::Rgb(0, 0, 0),
            ),
            ThemeMode::Light => (
                Color::Rgb(245, 240, 232),
                Color::Rgb(250, 246, 240),
                Color::Rgb(254, 251, 246),
                Color::Rgb(237, 231, 220),
            ),
        };

        let (text_primary, text_secondary, text_tertiary, text_disabled) = match mode {
            ThemeMode::Dark => (
                Color::Rgb(250, 250, 250),
                Color::Rgb(161, 161, 170),
                Color::Rgb(113, 113, 122),
                Color::Rgb(63, 63, 70),
            ),
            ThemeMode::Light => (
                Color::Rgb(28, 25, 23),
                Color::Rgb(87, 83, 78),
                Color::Rgb(135, 130, 124),
                Color::Rgb(214, 211, 209),
            ),
        };

        let (border_default, border_strong) = match mode {
            ThemeMode::Dark => (Color::Rgb(26, 26, 26), Color::Rgb(41, 41, 41)),
            ThemeMode::Light => (Color::Rgb(221, 214, 202), Color::Rgb(207, 198, 184)),
        };

        let (success, danger, warning, info) = match mode {
            ThemeMode::Dark => (
                Color::Rgb(74, 222, 128),
                Color::Rgb(248, 113, 113),
                Color::Rgb(251, 191, 36),
                Color::Rgb(161, 161, 170),
            ),
            ThemeMode::Light => (
                Color::Rgb(16, 185, 129),
                Color::Rgb(239, 68, 68),
                Color::Rgb(245, 158, 11),
                Color::Rgb(113, 113, 122),
            ),
        };

        Self {
            mode,
            accent_type: accent,
            bg_page,
            bg_surface,
            bg_elevated,
            bg_sunken,
            text_primary,
            text_secondary,
            text_tertiary,
            text_disabled,
            border_default,
            border_strong,
            accent: accent.color(mode),
            accent_muted: accent.muted(mode),
            success,
            danger,
            warning,
            info,
        }
    }

    pub fn from_config(config: &nyzhi_config::TuiConfig) -> Self {
        let mode = match config.theme.as_str() {
            "light" => ThemeMode::Light,
            _ => ThemeMode::Dark,
        };
        let accent = Accent::from_name(&config.accent);
        Self::new(mode, accent)
    }

    pub fn toggle_mode(&mut self) {
        let new_mode = match self.mode {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        };
        *self = Self::new(new_mode, self.accent_type);
    }

    pub fn next_accent(&mut self) {
        let next = self.accent_type.next();
        *self = Self::new(self.mode, next);
    }
}
