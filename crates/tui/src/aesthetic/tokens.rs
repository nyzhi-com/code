/// Spacing scale. Terminal cells are ~2:1 (taller than wide),
/// so horizontal values are doubled for visual symmetry.

pub const SP_0: u16 = 0;
pub const SP_1: u16 = 1;
pub const SP_2: u16 = 2;
pub const SP_4: u16 = 4;
pub const SP_8: u16 = 8;

pub const PAD_H: u16 = 2;
pub const PAD_H_LG: u16 = 4;

pub const ACCENT_BAR_W: u16 = 1;
pub const ACCENT_BAR_GAP: u16 = 1;
pub const ACCENT_GUTTER: u16 = ACCENT_BAR_W + ACCENT_BAR_GAP;

pub const INPUT_MIN_H: u16 = 5;
pub const INPUT_MAX_H: u16 = 12;
pub const HEADER_H: u16 = 1;
pub const FOOTER_H: u16 = 1;
pub const STATUS_BAR_H: u16 = 1;

pub const POPUP_MIN_W: u16 = 30;
pub const POPUP_MAX_W_PCT: u16 = 80;
pub const POPUP_MARGIN: u16 = 4;

pub const SIDE_PANEL_PCT: u16 = 35;
pub const SIDE_PANEL_MIN_W: u16 = 30;
pub const NARROW_THRESHOLD: u16 = 60;

/// Chat indent levels (in chars).
pub const INDENT_1: usize = 2;
pub const INDENT_2: usize = 4;
pub const INDENT_3: usize = 6;

/// Max line width before truncation in chat content.
pub const MAX_LINE_W: usize = 120;
