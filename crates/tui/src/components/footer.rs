use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::aesthetic::tokens::*;
use crate::aesthetic::typography as ty;
use crate::app::{App, AppMode};
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let mut hints: Vec<(&str, &str)> = Vec::new();

    match app.mode {
        AppMode::Streaming => {
            hints.push(("esc", "cancel"));
        }
        AppMode::AwaitingApproval => {
            hints.push(("\u{2190}\u{2192}", "select"));
            hints.push(("enter", "confirm"));
        }
        AppMode::AwaitingUserQuestion => {
            hints.push(("\u{2191}\u{2193}", "select"));
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
            right_spans.push(Span::raw(" ".repeat(SP_2 as usize)));
        }
        right_spans.push(Span::styled(
            (*key).to_string(),
            ty::secondary(theme).bold(),
        ));
        right_spans.push(Span::styled(
            format!(" {label}"),
            ty::disabled(theme),
        ));
    }

    let right_len: usize = right_spans.iter().map(|s| s.width()).sum();
    let gap = (area.width as usize).saturating_sub(right_len + PAD_H as usize);

    let mut spans = vec![Span::raw(" ".repeat(gap.max(1)))];
    spans.extend(right_spans);
    spans.push(Span::raw(" "));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(ty::on_page(theme));
    frame.render_widget(paragraph, area);
}
