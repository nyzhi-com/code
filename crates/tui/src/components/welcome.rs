use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::aesthetic::primitives;
use crate::aesthetic::tokens::*;
use crate::aesthetic::typography as ty;
use crate::app::App;
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    frame.render_widget(
        Block::default().style(ty::on_page(theme)),
        area,
    );

    let logo_rows = app.logo_anim.rows() as u16;
    let logo_cols = app.logo_anim.cols() as u16;
    let logo_color = app.logo_anim.breathing_color(theme.accent);
    let logo_frame = app.logo_anim.current_frame();

    let subtitle = format!("code  v{}", env!("CARGO_PKG_VERSION"));

    let auth = nyzhi_auth::auth_status(&app.provider_name);
    let status_text = if auth == "not connected" {
        "type /connect to get started".to_string()
    } else {
        format!("{} \u{00B7} {}", app.provider_name, app.model_name)
    };

    let shortcuts: &[(&str, &str)] = &[
        ("/", "commands"),
        ("S-Tab", "plan"),
        ("Tab", "thinking"),
        ("S-Enter", "newline"),
    ];

    let hint_width: u16 = shortcuts
        .iter()
        .map(|(k, d)| k.len() + 1 + d.len() + SP_4 as usize)
        .sum::<usize>() as u16;

    let content_lines = logo_rows + 7;
    let card_inner_w = logo_cols
        .max(hint_width)
        .max(subtitle.len() as u16)
        .max(status_text.len() as u16)
        + PAD_H * 2;
    let card_w = (card_inner_w + 2).min(area.width.saturating_sub(4));
    let card_h = (content_lines + SP_2 * 2 + 2).min(area.height.saturating_sub(2));

    let card_area = primitives::centered_popup(area, card_w, card_h);

    let card = primitives::Card::new(theme)
        .bg_color(theme.bg_surface)
        .border(theme.border_default);
    let inner = card.render_frame(frame, card_area);

    let padded = Rect::new(
        inner.x + PAD_H,
        inner.y,
        inner.width.saturating_sub(PAD_H * 2),
        inner.height,
    );

    let inner_w = padded.width as usize;

    let content_h = logo_rows + 7;
    let vert_pad = padded.height.saturating_sub(content_h) / 2;

    let mut lines: Vec<Line> = Vec::new();

    for _ in 0..vert_pad {
        lines.push(Line::from(""));
    }

    for logo_line in &logo_frame {
        let display_w = logo_line.chars().count();
        let pad = inner_w.saturating_sub(display_w) / 2;
        lines.push(Line::from(Span::styled(
            format!("{:>pad$}{logo_line}", ""),
            Style::default().fg(logo_color),
        )));
    }

    lines.push(Line::from(""));

    let sub_pad = inner_w.saturating_sub(subtitle.len()) / 2;
    lines.push(Line::from(Span::styled(
        format!("{:>sub_pad$}{subtitle}", ""),
        ty::disabled(theme),
    )));

    lines.push(Line::from(""));

    let status_style = if auth == "not connected" {
        ty::warning_style(theme)
    } else {
        ty::secondary(theme)
    };
    let s_pad = inner_w.saturating_sub(status_text.len()) / 2;
    lines.push(Line::from(Span::styled(
        format!("{:>s_pad$}{status_text}", ""),
        status_style,
    )));

    lines.push(Line::from(""));
    lines.push(Line::from(""));

    let mut hint_spans: Vec<Span> = Vec::new();
    for (i, (key, desc)) in shortcuts.iter().enumerate() {
        if i > 0 {
            hint_spans.push(Span::styled(
                "  \u{00B7}  ",
                Style::default().fg(theme.border_default),
            ));
        }
        hint_spans.push(Span::styled(
            format!(" {key} "),
            Style::default()
                .fg(theme.text_primary)
                .bg(theme.bg_elevated)
                .bold(),
        ));
        hint_spans.push(Span::styled(
            format!(" {desc}"),
            ty::disabled(theme),
        ));
    }

    let total_hint_w: usize = hint_spans.iter().map(|s| s.width()).sum();
    let ht_pad = inner_w.saturating_sub(total_hint_w) / 2;
    let mut padded_hints = vec![Span::raw(" ".repeat(ht_pad))];
    padded_hints.extend(hint_spans);
    lines.push(Line::from(padded_hints));

    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(theme.bg_surface));
    frame.render_widget(paragraph, padded);
}
