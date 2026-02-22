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
    let mut left = match app.mode {
        AppMode::Input => "enter send".to_string(),
        AppMode::Streaming => "ctrl+b background  esc interrupt".to_string(),
        AppMode::AwaitingApproval => "y approve  n deny".to_string(),
    };

    if app.mode == AppMode::Streaming {
        if let Some(start) = &app.stream_start {
            let elapsed = start.elapsed();
            if elapsed.as_millis() > 500 && app.stream_token_count > 0 {
                let tok_s = app.stream_token_count as f64 / elapsed.as_secs_f64();
                left = format!("{left}  {:.0} tok/s", tok_s);
            }
        }
    }

    let usage = &app.session_usage;
    let total_tokens = usage.total_input_tokens + usage.total_output_tokens;

    let turn_tokens = usage.turn_input_tokens as u64 + usage.turn_output_tokens as u64;
    let turn_cached = usage.turn_cache_read_tokens as u64;
    let usage_str = if total_tokens > 0 {
        if turn_tokens > 0 {
            let turn_str = if turn_cached > 0 {
                format!(
                    "turn: {} ({} cached)",
                    format_tokens(turn_tokens),
                    format_tokens(turn_cached),
                )
            } else {
                format!("turn: {}", format_tokens(turn_tokens))
            };
            format!(
                "{}  total: {}tok  {}",
                turn_str,
                format_tokens(total_tokens),
                format_cost(usage.total_cost_usd),
            )
        } else {
            format!(
                "{}tok  {}",
                format_tokens(total_tokens),
                format_cost(usage.total_cost_usd),
            )
        }
    } else {
        String::new()
    };

    let mut right_parts: Vec<&str> = Vec::new();
    let usage_owned;
    if !usage_str.is_empty() {
        usage_owned = usage_str;
        right_parts.push(&usage_owned);
    }

    let project_label;
    if let Some(pt) = &app.workspace.project_type {
        project_label = pt.name().to_string();
        right_parts.push(&project_label);
    }

    let branch_label;
    if let Some(branch) = &app.workspace.git_branch {
        branch_label = branch.clone();
        right_parts.push(&branch_label);
    }

    right_parts.push(&app.provider_name);
    right_parts.push(&app.model_name);
    let theme_label = match theme.mode {
        crate::theme::ThemeMode::Dark => "dark",
        crate::theme::ThemeMode::Light => "light",
    };
    right_parts.push(theme_label);

    let right = right_parts.join("  ");

    let shortcuts = "ctrl+t theme  ctrl+a accent  /model switch";

    let available = area.width as usize;
    let right_len = right.len();
    let left_len = left.len();
    let short_len = shortcuts.len();
    let total = left_len + short_len + right_len + 4;

    let trust_span = match app.trust_mode {
        nyzhi_config::TrustMode::Full => Some(Span::styled(
            "TRUST:FULL  ",
            Style::default().fg(theme.danger).bold(),
        )),
        nyzhi_config::TrustMode::Limited => Some(Span::styled(
            "TRUST:LIMITED  ",
            Style::default().fg(theme.warning).bold(),
        )),
        nyzhi_config::TrustMode::Off => None,
    };

    let notify_span = if app.notify.desktop {
        Some(Span::styled(
            "notify  ",
            Style::default().fg(theme.text_disabled),
        ))
    } else {
        None
    };

    let bg_count = app.background_tasks.len();
    let bg_span = if bg_count > 0 {
        Some(Span::styled(
            format!("bg:{bg_count}  "),
            Style::default().fg(theme.accent).bold(),
        ))
    } else {
        None
    };

    let ctx_span = if app.context_window > 0 {
        let pct = (app.context_used_tokens as f64 / app.context_window as f64) * 100.0;
        let color = if pct >= 85.0 {
            theme.danger
        } else if pct >= 60.0 {
            theme.warning
        } else {
            theme.success
        };
        Some(Span::styled(
            format!(
                "ctx:{:.0}% ({}/{})  ",
                pct,
                format_tokens(app.context_used_tokens as u64),
                format_tokens(app.context_window as u64),
            ),
            Style::default().fg(color),
        ))
    } else {
        None
    };

    let line = if total <= available {
        let mut spans = vec![
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
        ];
        if let Some(ts) = trust_span {
            spans.push(ts);
        }
        if let Some(ns) = &notify_span {
            spans.push(ns.clone());
        }
        if let Some(bs) = &bg_span {
            spans.push(bs.clone());
        }
        if let Some(cs) = &ctx_span {
            spans.push(cs.clone());
        }
        spans.push(Span::styled(
            format!("{right}  "),
            Style::default().fg(theme.text_tertiary),
        ));
        Line::from(spans)
    } else {
        let mut spans = vec![Span::styled(
            format!("  {left}"),
            Style::default().fg(theme.text_tertiary),
        )];
        if let Some(ts) = trust_span {
            spans.push(Span::raw("  "));
            spans.push(ts);
        }
        if let Some(ns) = &notify_span {
            spans.push(ns.clone());
        }
        if let Some(bs) = &bg_span {
            spans.push(bs.clone());
        }
        if let Some(cs) = &ctx_span {
            spans.push(cs.clone());
        }
        spans.push(Span::styled(
            format!("{:>width$}  ", right, width = available.saturating_sub(left_len + 4)),
            Style::default().fg(theme.text_tertiary),
        ));
        Line::from(spans)
    };

    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.bg_page));
    frame.render_widget(paragraph, area);
}
