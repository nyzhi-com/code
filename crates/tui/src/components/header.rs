use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::aesthetic::borders;
use crate::aesthetic::tokens::*;
use crate::aesthetic::typography as ty;
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

    // -- right side: tokens, context %, cost
    let mut right_spans: Vec<Span> = Vec::new();

    if total_tokens > 0 {
        right_spans.push(Span::styled(format_tokens(total_tokens), ty::disabled(theme)));
    }

    if app.context_window > 0 && app.context_used_tokens > 0 {
        let pct = (app.context_used_tokens as f64 / app.context_window as f64 * 100.0) as u8;
        let pct_color = if pct >= 75 {
            theme.danger
        } else if pct >= 50 {
            theme.warning
        } else {
            theme.text_disabled
        };
        if !right_spans.is_empty() {
            right_spans.push(Span::raw("  "));
        }
        right_spans.push(Span::styled(format!("{pct}%"), Style::default().fg(pct_color)));
    }

    let cost = format_cost(usage.total_cost_usd);
    if !cost.is_empty() {
        if !right_spans.is_empty() {
            right_spans.push(Span::raw("  "));
        }
        right_spans.push(Span::styled(cost, ty::disabled(theme)));
    }
    right_spans.push(Span::raw(" "));

    let right_len: usize = right_spans.iter().map(|s| s.width()).sum();

    // -- left side: accent bar + title
    let title = &app.session_title;
    let title_max = w.saturating_sub(right_len + PAD_H as usize + ACCENT_GUTTER as usize + 2);
    let title_display: String = if title.len() > title_max {
        format!("{}...", &title[..title_max.saturating_sub(3)])
    } else {
        title.clone()
    };
    let left_len = title_display.len() + ACCENT_GUTTER as usize + 1;
    let gap = w.saturating_sub(left_len + right_len);

    let mut spans: Vec<Span> = vec![
        Span::styled(
            format!(" {} ", borders::BAR_CHAR),
            Style::default().fg(theme.accent),
        ),
        Span::styled(title_display, ty::heading(theme)),
        Span::raw(" ".repeat(gap.max(1))),
    ];
    spans.extend(right_spans);

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(ty::on_surface(theme));
    frame.render_widget(paragraph, area);
}
