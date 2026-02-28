use ratatui::style::Color;

pub const NYZHI_ORANGE: Color = Color::Rgb(0xEE, 0x60, 0x18);
pub const NYZHI_ORANGE_HEX: &str = "#EE6018";

pub const PITCH_BLACK: Color = Color::Rgb(0, 0, 0);
pub const NEAR_BLACK: Color = Color::Rgb(0x0A, 0x0A, 0x0C);
pub const DARK_SURFACE: Color = Color::Rgb(0x14, 0x14, 0x18);
pub const DARK_ELEVATED: Color = Color::Rgb(0x1C, 0x1C, 0x22);
pub const NEAR_WHITE: Color = Color::Rgb(0xFA, 0xFA, 0xFA);

pub const BORDER_SUBTLE: Color = Color::Rgb(0x1E, 0x1E, 0x26);
pub const BORDER_MEDIUM: Color = Color::Rgb(0x2C, 0x2C, 0x36);

pub const TEXT_SECONDARY: Color = Color::Rgb(0xA1, 0xA1, 0xAA);
pub const TEXT_TERTIARY: Color = Color::Rgb(0x71, 0x71, 0x7A);
pub const TEXT_DISABLED: Color = Color::Rgb(0x3F, 0x3F, 0x46);

pub const SUCCESS: Color = Color::Rgb(0x4A, 0xDE, 0x80);
pub const DANGER: Color = Color::Rgb(0xF8, 0x71, 0x71);
pub const WARNING: Color = Color::Rgb(0xFB, 0xBF, 0x24);
pub const INFO: Color = Color::Rgb(0xA1, 0xA1, 0xAA);

pub fn orange_muted() -> Color {
    let a = 64u16;
    let r = (0xEE_u16 * a / 255) as u8;
    let g = (0x60_u16 * a / 255) as u8;
    let b = (0x18_u16 * a / 255) as u8;
    Color::Rgb(r, g, b)
}
