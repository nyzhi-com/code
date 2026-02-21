use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, DisplayItem, ToolStatus};
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_default))
        .title(Line::from(vec![
            Span::styled(" nyzhi code ", Style::default().fg(theme.accent).bold()),
        ]))
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg_page));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    for item in &app.items {
        match item {
            DisplayItem::Message { role, content } => {
                render_message(&mut lines, role, content, theme);
            }
            DisplayItem::ToolCall {
                name,
                args_summary,
                output,
                status,
            } => {
                render_tool_call(&mut lines, name, args_summary, output, status, theme);
            }
        }
    }

    if !app.current_stream.is_empty() {
        lines.push(Line::from(""));
        for line in app.current_stream.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {line}"),
                Style::default().fg(theme.text_primary),
            )));
        }
        lines.push(Line::from(Span::styled(
            "  _",
            Style::default().fg(theme.accent),
        )));
    }

    let total_lines = lines.len() as u16;
    let visible = inner.height;
    let auto_scroll = total_lines.saturating_sub(visible);

    let scroll = if app.scroll_offset > 0 {
        auto_scroll.saturating_sub(app.scroll_offset)
    } else {
        auto_scroll
    };

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0))
        .style(Style::default().bg(theme.bg_page));

    frame.render_widget(paragraph, inner);
}

fn render_message<'a>(lines: &mut Vec<Line<'a>>, role: &str, content: &str, theme: &Theme) {
    lines.push(Line::from(""));

    match role {
        "user" => {
            lines.push(Line::from(Span::styled(
                format!("  {content}"),
                Style::default().fg(theme.text_primary).bold(),
            )));
        }
        _ => {
            for line in content.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  {line}"),
                    Style::default().fg(theme.text_primary),
                )));
            }
        }
    }
}

fn render_tool_call<'a>(
    lines: &mut Vec<Line<'a>>,
    name: &str,
    args_summary: &str,
    output: &Option<String>,
    status: &ToolStatus,
    theme: &Theme,
) {
    let (icon, icon_color) = match status {
        ToolStatus::Running => ("*", theme.warning),
        ToolStatus::WaitingApproval => ("?", theme.accent),
        ToolStatus::Completed => ("+", theme.success),
        ToolStatus::Denied => ("x", theme.danger),
    };

    let mut summary_lines = args_summary.lines();
    let first_line = summary_lines.next().unwrap_or("");

    let summary = if first_line.len() > 60 {
        format!("{}...", &first_line[..57])
    } else {
        first_line.to_string()
    };

    lines.push(Line::from(vec![
        Span::styled(
            format!("    {icon} "),
            Style::default().fg(icon_color),
        ),
        Span::styled(
            name.to_string(),
            Style::default().fg(theme.accent).bold(),
        ),
        Span::styled(
            format!(" {summary}"),
            Style::default().fg(theme.text_tertiary),
        ),
    ]));

    if *status == ToolStatus::WaitingApproval {
        for diff_line in summary_lines {
            lines.push(render_diff_line(diff_line, theme));
        }
    }

    if let Some(out) = output {
        for line in out.lines().take(3) {
            lines.push(Line::from(Span::styled(
                format!("      {line}"),
                Style::default().fg(theme.text_secondary),
            )));
        }
    }
}

fn render_diff_line<'a>(line: &str, theme: &Theme) -> Line<'a> {
    let color = if line.starts_with('+') {
        theme.success
    } else if line.starts_with('-') {
        theme.danger
    } else if line.starts_with("@@") {
        theme.info
    } else {
        theme.text_tertiary
    };
    Line::from(Span::styled(
        format!("      {line}"),
        Style::default().fg(color),
    ))
}
