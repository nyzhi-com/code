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
        return String::new();
    }
    if usd < 1.0 {
        format!("${:.3}", usd)
    } else {
        format!("${:.2}", usd)
    }
}

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let left = match app.mode {
        AppMode::Streaming => "esc cancel",
        AppMode::AwaitingApproval => "y approve  n deny",
        AppMode::AwaitingUserQuestion => "select an option",
        AppMode::Input => "^K cmds  Tab thinking",
    };

    let mut right_parts: Vec<String> = Vec::new();

    if let nyzhi_config::TrustMode::Full = app.trust_mode {
        right_parts.push("TRUST:FULL".to_string());
    }

    let bg_count = app.background_tasks.len();
    if bg_count > 0 {
        right_parts.push(format!("bg:{bg_count}"));
    }

    let usage = &app.session_usage;
    let total_tokens = usage.total_input_tokens + usage.total_output_tokens;
    if total_tokens > 0 {
        let cost = format_cost(usage.total_cost_usd);
        let tok = format!("{}tok", format_tokens(total_tokens));
        if cost.is_empty() {
            right_parts.push(tok);
        } else {
            right_parts.push(format!("{tok} {cost}"));
        }
    }

    if let Some(ref level) = app.thinking_level {
        right_parts.push(format!("think:{level}"));
    }

    let auth = nyzhi_auth::auth_status(&app.provider_name);
    if auth == "not connected" {
        right_parts.push("not connected".to_string());
    } else {
        right_parts.push(format!("{} {}", app.provider_name, app.model_name));
    }

    let right = right_parts.join("  ");

    let available = area.width as usize;
    let gap = available.saturating_sub(left.len() + right.len() + 4);

    let mut spans: Vec<Span> = Vec::new();

    if !left.is_empty() {
        spans.push(Span::styled(
            format!("  {left}"),
            Style::default().fg(theme.text_tertiary),
        ));
    }

    spans.push(Span::raw(" ".repeat(if left.is_empty() { available.saturating_sub(right.len() + 2) } else { gap })));

    if matches!(app.trust_mode, nyzhi_config::TrustMode::Full) {
        let trust_part = "TRUST:FULL  ";
        let rest = right.strip_prefix("TRUST:FULL  ").unwrap_or(&right);
        spans.push(Span::styled(trust_part, Style::default().fg(theme.danger).bold()));
        spans.push(Span::styled(format!("{rest}  "), Style::default().fg(theme.text_tertiary)));
    } else {
        spans.push(Span::styled(format!("{right}  "), Style::default().fg(theme.text_tertiary)));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.bg_page));
    frame.render_widget(paragraph, area);
}
