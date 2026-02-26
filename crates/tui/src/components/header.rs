use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::App;
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
    let w = area.width as usize;

    let usage = &app.session_usage;
    let total_tokens = usage.total_input_tokens + usage.total_output_tokens;
    let mut right_parts: Vec<String> = Vec::new();
    if total_tokens > 0 {
        right_parts.push(format_tokens(total_tokens));
    }
    if app.context_window > 0 && app.context_used_tokens > 0 {
        let pct = (app.context_used_tokens as f64 / app.context_window as f64 * 100.0) as u8;
        let pct_color = if pct >= 75 {
            theme.danger
        } else if pct >= 50 {
            theme.warning
        } else {
            theme.text_tertiary
        };
        right_parts.push(format!("{}%", pct));
        let _ = pct_color; // used below in span construction
    }
    let cost = format_cost(usage.total_cost_usd);
    if !cost.is_empty() {
        right_parts.push(cost.clone());
    }
    let right = right_parts.join("  ");
    let right_len = right.len() + 1;

    let title = &app.session_title;
    let title_max = w.saturating_sub(right_len + 5);
    let title_display: String = if title.len() > title_max {
        format!("{}...", &title[..title_max.saturating_sub(3)])
    } else {
        title.clone()
    };
    let left_len = title_display.len() + 4;
    let gap = w.saturating_sub(left_len + right_len);

    let ctx_pct = if app.context_window > 0 && app.context_used_tokens > 0 {
        Some((app.context_used_tokens as f64 / app.context_window as f64 * 100.0) as u8)
    } else {
        None
    };

    let mut right_spans: Vec<Span> = Vec::new();
    if total_tokens > 0 {
        right_spans.push(Span::styled(
            format_tokens(total_tokens),
            Style::default().fg(theme.text_disabled),
        ));
    }
    if let Some(pct) = ctx_pct {
        if !right_spans.is_empty() {
            right_spans.push(Span::styled("  ", Style::default()));
        }
        let pct_color = if pct >= 75 {
            theme.danger
        } else if pct >= 50 {
            theme.warning
        } else {
            theme.text_disabled
        };
        right_spans.push(Span::styled(
            format!("{pct}%"),
            Style::default().fg(pct_color),
        ));
    }
    if !cost.is_empty() {
        if !right_spans.is_empty() {
            right_spans.push(Span::styled("  ", Style::default()));
        }
        right_spans.push(Span::styled(
            format_cost(usage.total_cost_usd),
            Style::default().fg(theme.text_disabled),
        ));
    }
    right_spans.push(Span::raw(" "));

    let mut spans: Vec<Span> = vec![
        Span::styled(" ┃ ", Style::default().fg(theme.accent)),
        Span::styled(
            title_display,
            Style::default().fg(theme.text_primary).bold(),
        ),
        Span::raw(" ".repeat(gap.max(1))),
    ];
    spans.extend(right_spans);

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.bg_surface));
    frame.render_widget(paragraph, area);
}
