use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::theme::Theme;

pub const BAR_CHAR: &str = "┃";
pub const THIN_H: &str = "─";

pub fn accent_bar(frame: &mut ratatui::Frame, area: Rect, color: Color, bg: Color) {
    for row in 0..area.height {
        frame.render_widget(
            Paragraph::new(Span::styled(BAR_CHAR, Style::default().fg(color)))
                .style(Style::default().bg(bg)),
            Rect::new(area.x, area.y + row, 1, 1),
        );
    }
}

pub fn h_separator(frame: &mut ratatui::Frame, area: Rect, theme: &Theme) {
    let line = THIN_H.repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(line).style(
            Style::default()
                .fg(theme.border_default)
                .bg(theme.bg_surface),
        ),
        area,
    );
}

pub fn h_separator_label<'a>(
    label: &str,
    width: u16,
    theme: &Theme,
) -> Line<'a> {
    let label_len = label.len() + 2;
    let side = (width as usize).saturating_sub(label_len) / 2;
    let right = (width as usize).saturating_sub(side + label_len);
    let mut spans = Vec::new();
    spans.push(Span::styled(
        THIN_H.repeat(side),
        Style::default().fg(theme.border_default),
    ));
    spans.push(Span::styled(
        format!(" {label} "),
        Style::default().fg(theme.text_tertiary),
    ));
    spans.push(Span::styled(
        THIN_H.repeat(right),
        Style::default().fg(theme.border_default),
    ));
    Line::from(spans)
}
