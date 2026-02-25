use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, AppMode};
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let context_hints = match app.mode {
        AppMode::Streaming => "esc cancel",
        AppMode::AwaitingApproval => "←→ select  enter confirm",
        AppMode::AwaitingUserQuestion => "↑↓ select  enter confirm",
        AppMode::Input => "S-Tab mode  ^P cmds  tab model",
    };

    let mut right_parts: Vec<String> = Vec::new();

    if matches!(app.trust_mode, nyzhi_config::TrustMode::Full) {
        right_parts.push("TRUST:FULL".to_string());
    }

    let bg_count = app.background_tasks.len();
    if bg_count > 0 {
        right_parts.push(format!("bg:{bg_count}"));
    }

    if let Some((indexed, total, complete)) = app.index_progress {
        if complete {
            if let Some(ref err) = app.index_error {
                let short = if err.len() > 30 { &err[..30] } else { err };
                right_parts.push(format!("idx:⚠ {short}"));
            }
        } else if total > 0 {
            right_parts.push(format!("idx:{indexed}/{total}"));
        } else {
            right_parts.push("idx:…".to_string());
        }
    }

    let right_str = right_parts.join("  ");
    let available = area.width as usize;
    let gap = available.saturating_sub(context_hints.len() + right_str.len() + 3);

    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(
        format!(" {context_hints}"),
        Style::default().fg(theme.text_disabled),
    ));
    spans.push(Span::raw(" ".repeat(gap.max(1))));

    if !right_parts.is_empty() {
        if right_parts.iter().any(|p| p == "TRUST:FULL") {
            spans.push(Span::styled(
                "TRUST:FULL",
                Style::default().fg(theme.danger).bold(),
            ));
            let rest: Vec<&str> = right_parts
                .iter()
                .filter(|p| *p != "TRUST:FULL")
                .map(|s| s.as_str())
                .collect();
            if !rest.is_empty() {
                spans.push(Span::styled(
                    format!("  {}", rest.join("  ")),
                    Style::default().fg(theme.text_tertiary),
                ));
            }
        } else {
            spans.push(Span::styled(
                right_str,
                Style::default().fg(theme.text_tertiary),
            ));
        }
        spans.push(Span::raw(" "));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.bg_surface));
    frame.render_widget(paragraph, area);
}
