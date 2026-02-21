use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;
use crate::logo::LOGO_SPLASH;
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_default))
        .title(Line::from(vec![Span::styled(
            " nyzhi code ",
            Style::default().fg(theme.accent).bold(),
        )]))
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg_page));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    let logo_lines: Vec<&str> = LOGO_SPLASH.lines().collect();
    let logo_width = logo_lines.iter().map(|l| l.len()).max().unwrap_or(0);
    let inner_w = inner.width as usize;

    let vert_pad = inner
        .height
        .saturating_sub(logo_lines.len() as u16 + 14)
        / 2;

    for _ in 0..vert_pad {
        lines.push(Line::from(""));
    }

    for logo_line in &logo_lines {
        let pad = inner_w.saturating_sub(logo_width) / 2;
        lines.push(Line::from(Span::styled(
            format!("{:>pad$}{logo_line}", ""),
            Style::default().fg(theme.accent),
        )));
    }

    lines.push(Line::from(""));

    let title = "nyzhi code";
    let title_pad = inner_w.saturating_sub(title.len()) / 2;
    lines.push(Line::from(Span::styled(
        format!("{:>title_pad$}{title}", ""),
        Style::default().fg(theme.accent).bold(),
    )));

    let version = "v0.1.0";
    let ver_pad = inner_w.saturating_sub(version.len()) / 2;
    lines.push(Line::from(Span::styled(
        format!("{:>ver_pad$}{version}", ""),
        Style::default().fg(theme.text_tertiary),
    )));

    lines.push(Line::from(""));
    lines.push(Line::from(""));

    let commands: &[(&str, &str, &str)] = &[
        ("/help", "show help", "ctrl+h"),
        ("/clear", "clear session", "ctrl+l"),
        ("/theme", "choose theme", "ctrl+t"),
        ("/accent", "choose accent", "ctrl+a"),
        ("/model", "choose model", ""),
        ("/quit", "exit", "ctrl+c"),
    ];

    for &(cmd, desc, shortcut) in commands {
        let cmd_pad = inner_w.saturating_sub(46) / 2;
        lines.push(Line::from(vec![
            Span::raw(format!("{:>cmd_pad$}", "")),
            Span::styled(
                format!("{cmd:<14}"),
                Style::default().fg(theme.accent),
            ),
            Span::styled(
                format!("{desc:<20}"),
                Style::default().fg(theme.text_secondary),
            ),
            Span::styled(shortcut, Style::default().fg(theme.text_tertiary)),
        ]));
    }

    let footer = format!("{} {}", app.provider_name, app.model_name);
    let f_pad = inner_w.saturating_sub(footer.len()) / 2;
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("{:>f_pad$}{footer}", ""),
        Style::default().fg(theme.text_disabled),
    )));

    let paragraph = Paragraph::new(lines).style(Style::default().bg(theme.bg_page));
    frame.render_widget(paragraph, inner);
}
