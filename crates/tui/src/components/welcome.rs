use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;
use crate::logo::LOGO_SPLASH;
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let block = Block::default().style(Style::default().bg(theme.bg_page));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    let logo_lines: Vec<&str> = LOGO_SPLASH.lines().filter(|l| !l.is_empty()).collect();
    let logo_width = logo_lines.iter().map(|l| l.len()).max().unwrap_or(0);
    let inner_w = inner.width as usize;

    let content_height = logo_lines.len() + 4;
    let vert_pad = inner.height.saturating_sub(content_height as u16) / 2;

    for _ in 0..vert_pad {
        lines.push(Line::from(""));
    }

    for logo_line in &logo_lines {
        let pad = inner_w.saturating_sub(logo_width) / 2;
        lines.push(Line::from(Span::styled(
            format!("{:>pad$}{logo_line}", ""),
            Style::default().fg(theme.accent).bold(),
        )));
    }

    lines.push(Line::from(""));

    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let ver_pad = inner_w.saturating_sub(version.len()) / 2;
    lines.push(Line::from(Span::styled(
        format!("{:>ver_pad$}{version}", ""),
        Style::default().fg(theme.text_tertiary),
    )));

    lines.push(Line::from(""));

    let auth = nyzhi_auth::auth_status(&app.provider_name);
    let footer = if auth == "not connected" {
        "type /connect to get started".to_string()
    } else {
        format!("{} · {}", app.provider_name, app.model_name)
    };
    let f_pad = inner_w.saturating_sub(footer.len()) / 2;
    lines.push(Line::from(Span::styled(
        format!("{:>f_pad$}{footer}", ""),
        Style::default().fg(theme.text_disabled),
    )));

    let paragraph = Paragraph::new(lines).style(Style::default().bg(theme.bg_page));
    frame.render_widget(paragraph, inner);
}
