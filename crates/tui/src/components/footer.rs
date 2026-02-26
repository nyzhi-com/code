use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, AppMode};
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let mut hints: Vec<(&str, &str)> = Vec::new();

    match app.mode {
        AppMode::Streaming => {
            hints.push(("esc", "cancel"));
        }
        AppMode::AwaitingApproval => {
            hints.push(("←→", "select"));
            hints.push(("enter", "confirm"));
        }
        AppMode::AwaitingUserQuestion => {
            hints.push(("↑↓", "select"));
            hints.push(("enter", "confirm"));
        }
        AppMode::Input => {
            hints.push(("tab", "agents"));
            hints.push(("ctrl+p", "commands"));
        }
    }

    let mut right_spans: Vec<Span> = Vec::new();
    for (i, (key, label)) in hints.iter().enumerate() {
        if i > 0 {
            right_spans.push(Span::styled("  ", Style::default()));
        }
        right_spans.push(Span::styled(
            (*key).to_string(),
            Style::default().fg(theme.text_secondary).bold(),
        ));
        right_spans.push(Span::styled(
            format!(" {label}"),
            Style::default().fg(theme.text_disabled),
        ));
    }

    let right_len: usize = right_spans.iter().map(|s| s.width()).sum();
    let gap = (area.width as usize).saturating_sub(right_len + 2);

    let mut spans = vec![Span::raw(" ".repeat(gap.max(1)))];
    spans.extend(right_spans);
    spans.push(Span::raw(" "));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.bg_page));
    frame.render_widget(paragraph, area);
}
