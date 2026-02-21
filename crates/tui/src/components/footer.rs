use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, AppMode};
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let left = match app.mode {
        AppMode::Input => "enter send",
        AppMode::Streaming => "esc interrupt",
        AppMode::AwaitingApproval => "y approve  n deny",
    };

    let right = format!(
        "{} {}  {}  {}",
        app.provider_name,
        app.model_name,
        theme.accent_type.name(),
        match theme.mode {
            crate::theme::ThemeMode::Dark => "dark",
            crate::theme::ThemeMode::Light => "light",
        }
    );

    let shortcuts = "ctrl+t theme  ctrl+a accent";

    let available = area.width as usize;
    let right_len = right.len();
    let left_len = left.len();
    let short_len = shortcuts.len();
    let total = left_len + short_len + right_len + 4;

    let line = if total <= available {
        Line::from(vec![
            Span::styled(
                format!("  {left}"),
                Style::default().fg(theme.text_tertiary),
            ),
            Span::styled(
                format!(
                    "{:^width$}",
                    shortcuts,
                    width = available - left_len - right_len - 4
                ),
                Style::default().fg(theme.text_disabled),
            ),
            Span::styled(
                format!("{right}  "),
                Style::default().fg(theme.text_tertiary),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!("  {left}"),
                Style::default().fg(theme.text_tertiary),
            ),
            Span::styled(
                format!("{:>width$}  ", right, width = available.saturating_sub(left_len + 4)),
                Style::default().fg(theme.text_tertiary),
            ),
        ])
    };

    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.bg_page));
    frame.render_widget(paragraph, area);
}
