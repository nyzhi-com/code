use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, theme: &Theme) {
    let text = Line::from(vec![
        Span::styled(" ┃ ", Style::default().fg(theme.warning).bold()),
        Span::styled(
            "Plan Mode",
            Style::default().fg(theme.warning).bold(),
        ),
        Span::styled(
            " — read-only analysis, no edits or commands",
            Style::default().fg(theme.text_secondary),
        ),
    ]);
    let paragraph = Paragraph::new(text).style(Style::default().bg(theme.bg_surface));
    frame.render_widget(paragraph, area);
}

pub fn height(plan_mode: bool) -> u16 {
    if plan_mode { 1 } else { 0 }
}
