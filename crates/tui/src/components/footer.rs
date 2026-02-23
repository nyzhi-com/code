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
    let mode_toggle_hint = if app.plan_mode { "S-Tab act" } else { "S-Tab plan" };
    let left = match app.mode {
        AppMode::Streaming => "esc cancel",
        AppMode::AwaitingApproval => "y approve  n deny",
        AppMode::AwaitingUserQuestion => "select an option",
        AppMode::Input => mode_toggle_hint,
    };

    let mut info_parts: Vec<String> = Vec::new();

    if let nyzhi_config::TrustMode::Full = app.trust_mode {
        info_parts.push("TRUST:FULL".to_string());
    }

    let bg_count = app.background_tasks.len();
    if bg_count > 0 {
        info_parts.push(format!("bg:{bg_count}"));
    }

    let usage = &app.session_usage;
    let total_tokens = usage.total_input_tokens + usage.total_output_tokens;
    if total_tokens > 0 {
        let cost = format_cost(usage.total_cost_usd);
        let tok = format!("{}tok", format_tokens(total_tokens));
        if cost.is_empty() {
            info_parts.push(tok);
        } else {
            info_parts.push(format!("{tok} {cost}"));
        }
    }

    if let Some(ref level) = app.thinking_level {
        info_parts.push(format!("think:{level}"));
    }

    if let Some((done, active, total)) = app.todo_progress {
        if total > 0 {
            let label = if active > 0 {
                format!("todos:{done}/{total} ({active})")
            } else {
                format!("todos:{done}/{total}")
            };
            info_parts.push(label);
        }
    }

    let queue_count = app.message_queue.len();
    if queue_count > 0 {
        info_parts.push(format!("queue:{queue_count}"));
    }

    let auth = nyzhi_auth::auth_status(&app.provider_name);
    let model_label = if auth == "not connected" {
        "not connected".to_string()
    } else {
        format!("{} {}", app.provider_name, app.model_name)
    };

    let info_str = info_parts.join("  ");
    let right_len = if info_str.is_empty() {
        model_label.len()
    } else {
        info_str.len() + 2 + model_label.len()
    };

    let mode_badge = if app.plan_mode { "Plan" } else { "Act" };
    let badge_len = mode_badge.len() + 3;

    let available = area.width as usize;
    let gap = available.saturating_sub(badge_len + left.len() + right_len + 4);

    let mut spans: Vec<Span> = Vec::new();

    if app.plan_mode {
        spans.push(Span::styled(
            format!(" {mode_badge} "),
            Style::default().fg(theme.bg_page).bg(theme.warning).bold(),
        ));
    } else {
        spans.push(Span::styled(
            format!(" {mode_badge} "),
            Style::default().fg(theme.text_tertiary),
        ));
    }

    if !left.is_empty() {
        spans.push(Span::styled(
            format!(" {left}"),
            Style::default().fg(theme.text_tertiary),
        ));
    }

    spans.push(Span::raw(" ".repeat(if left.is_empty() { available.saturating_sub(right_len + 2) } else { gap })));

    if matches!(app.trust_mode, nyzhi_config::TrustMode::Full) {
        let without_trust: Vec<&str> = info_parts.iter()
            .filter(|p| *p != "TRUST:FULL")
            .map(|s| s.as_str())
            .collect();
        spans.push(Span::styled("TRUST:FULL", Style::default().fg(theme.danger).bold()));
        if !without_trust.is_empty() {
            spans.push(Span::styled(
                format!("  {}", without_trust.join("  ")),
                Style::default().fg(theme.text_tertiary),
            ));
        }
    } else if !info_str.is_empty() {
        spans.push(Span::styled(info_str, Style::default().fg(theme.text_tertiary)));
    }

    if !info_parts.is_empty() || matches!(app.trust_mode, nyzhi_config::TrustMode::Full) {
        spans.push(Span::raw("  "));
    }

    let model_style = if auth == "not connected" {
        Style::default().fg(theme.text_disabled)
    } else {
        Style::default().fg(theme.accent).bold()
    };
    spans.push(Span::styled(format!("{model_label}  "), model_style));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.bg_surface));
    frame.render_widget(paragraph, area);
}
