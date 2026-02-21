use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, AppMode};
use crate::spinner::SpinnerState;
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme, spinner: &SpinnerState) {
    let border_color = match app.mode {
        AppMode::Input => theme.border_strong,
        _ => theme.border_default,
    };

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg_page));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content: Line = match app.mode {
        AppMode::Input => Line::from(vec![
            Span::styled("> ", Style::default().fg(theme.accent).bold()),
            Span::styled(&app.input, Style::default().fg(theme.text_primary)),
        ]),
        AppMode::Streaming => Line::from(vec![
            Span::styled(
                format!("{} ", spinner.current_frame()),
                Style::default().fg(theme.accent),
            ),
            Span::styled(
                "thinking...",
                Style::default().fg(theme.text_tertiary),
            ),
        ]),
        AppMode::AwaitingApproval => Line::from(vec![
            Span::styled("[y/n] ", Style::default().fg(theme.accent).bold()),
            Span::styled(
                "approve?",
                Style::default().fg(theme.text_secondary),
            ),
        ]),
    };

    let paragraph = Paragraph::new(content).style(Style::default().bg(theme.bg_page));
    frame.render_widget(paragraph, inner);

    if matches!(app.mode, AppMode::Input) {
        frame.set_cursor_position(Position::new(
            inner.x + 2 + app.cursor_pos as u16,
            inner.y,
        ));
    }
}
