use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, AppMode, DisplayItem, ToolStatus};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_header(frame, chunks[0]);
    draw_items(frame, chunks[1], app);
    draw_input(frame, chunks[2], app);
    draw_status(frame, chunks[3], app);
}

fn draw_header(frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(" nyzhi code")
        .style(Style::default().fg(Color::White).bg(Color::DarkGray).bold());
    frame.render_widget(header, area);
}

fn draw_items(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    for item in &app.items {
        match item {
            DisplayItem::Message { role, content } => {
                let (label, color) = match role.as_str() {
                    "user" => ("you", Color::Cyan),
                    "assistant" => ("nyzhi", Color::Green),
                    _ => ("system", Color::Yellow),
                };

                lines.push(Line::from(vec![Span::styled(
                    format!("{label} "),
                    Style::default().fg(color).bold(),
                )]));

                for line in content.lines() {
                    lines.push(Line::from(format!("  {line}")));
                }
                lines.push(Line::from(""));
            }
            DisplayItem::ToolCall {
                name,
                args_summary,
                output,
                status,
            } => {
                let (icon, color) = match status {
                    ToolStatus::Running => ("...", Color::Yellow),
                    ToolStatus::WaitingApproval => ("?", Color::Magenta),
                    ToolStatus::Completed => ("ok", Color::Green),
                    ToolStatus::Denied => ("x", Color::Red),
                };

                let summary = if args_summary.len() > 80 {
                    format!("{}...", &args_summary[..77])
                } else {
                    args_summary.clone()
                };

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  [{icon}] "),
                        Style::default().fg(color),
                    ),
                    Span::styled(
                        name.to_string(),
                        Style::default().fg(Color::Blue).bold(),
                    ),
                    Span::styled(
                        format!(" {summary}"),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));

                if let Some(out) = output {
                    for line in out.lines().take(5) {
                        lines.push(Line::from(Span::styled(
                            format!("    {line}"),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                }

                lines.push(Line::from(""));
            }
        }
    }

    if !app.current_stream.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "nyzhi ",
            Style::default().fg(Color::Green).bold(),
        )]));
        for line in app.current_stream.lines() {
            lines.push(Line::from(format!("  {line}")));
        }
    }

    let total_lines = lines.len() as u16;
    let visible = area.height;
    let auto_scroll = total_lines.saturating_sub(visible);

    let scroll = if app.scroll_offset > 0 {
        auto_scroll.saturating_sub(app.scroll_offset)
    } else {
        auto_scroll
    };

    let chat = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(chat, area);
}

fn draw_input(frame: &mut Frame, area: Rect, app: &App) {
    let prompt = match app.mode {
        AppMode::Input => "> ",
        AppMode::Streaming => "  streaming...",
        AppMode::AwaitingApproval => "  [y/n] ",
    };

    let input = Paragraph::new(format!("{prompt}{}", app.input)).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(input, area);

    if matches!(app.mode, AppMode::Input) {
        frame.set_cursor_position(Position::new(
            area.x + 2 + app.cursor_pos as u16,
            area.y + 1,
        ));
    }
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let status = Paragraph::new(format!(" {}", app.status))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(status, area);
}
