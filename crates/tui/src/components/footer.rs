use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, AppMode};
use crate::theme::Theme;

fn format_tokens(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

fn format_cost(usd: f64) -> String {
    if usd < 0.001 {
        return "$0.00".to_string();
    }
    if usd < 1.0 {
        format!("${:.3}", usd)
    } else {
        format!("${:.2}", usd)
    }
}

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let left = match app.mode {
        AppMode::Input => "enter send",
        AppMode::Streaming => "esc interrupt",
        AppMode::AwaitingApproval => "y approve  n deny",
    };

    let usage = &app.session_usage;
    let total_tokens = usage.total_input_tokens + usage.total_output_tokens;
    let usage_str = if total_tokens > 0 {
        format!(
            "{}tok  {}",
            format_tokens(total_tokens),
            format_cost(usage.total_cost_usd),
        )
    } else {
        String::new()
    };

    let right = format!(
        "{}{}{}  {}  {}",
        usage_str,
        if usage_str.is_empty() { "" } else { "  " },
        app.provider_name,
        app.model_name,
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
