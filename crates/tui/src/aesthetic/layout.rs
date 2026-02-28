use ratatui::prelude::*;

use super::tokens::*;

pub struct Regions {
    pub banner: Rect,
    pub plan_banner: Rect,
    pub header: Rect,
    pub main: Rect,
    pub input: Rect,
    pub footer: Rect,
}

pub fn compute(
    area: Rect,
    banner_h: u16,
    plan_h: u16,
    show_header: bool,
    input_lines: u16,
) -> Regions {
    let header_h = if show_header { HEADER_H } else { 0 };
    let input_h = (input_lines + STATUS_BAR_H + SP_1)
        .max(INPUT_MIN_H)
        .min(INPUT_MAX_H);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(banner_h),
            Constraint::Length(plan_h),
            Constraint::Length(header_h),
            Constraint::Min(1),
            Constraint::Length(input_h),
            Constraint::Length(FOOTER_H),
        ])
        .split(area);

    Regions {
        banner: chunks[0],
        plan_banner: chunks[1],
        header: chunks[2],
        main: chunks[3],
        input: chunks[4],
        footer: chunks[5],
    }
}

pub fn split_side_panel(main: Rect) -> (Rect, Option<Rect>) {
    if main.width < NARROW_THRESHOLD + SIDE_PANEL_MIN_W {
        return (main, None);
    }
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(100 - SIDE_PANEL_PCT),
            Constraint::Percentage(SIDE_PANEL_PCT),
        ])
        .split(main);
    (split[0], Some(split[1]))
}
