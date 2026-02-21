use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, AppMode};

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
    draw_messages(frame, chunks[1], app);
    draw_input(frame, chunks[2], app);
    draw_status(frame, chunks[3], app);
}

fn draw_header(frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(" nyzhi code")
        .style(Style::default().fg(Color::White).bg(Color::DarkGray).bold());
    frame.render_widget(header, area);
}

fn draw_messages(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        let (label, color) = match msg.role.as_str() {
            "user" => ("you", Color::Cyan),
            "assistant" => ("nyzhi", Color::Green),
            _ => ("system", Color::Yellow),
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{label} "), Style::default().fg(color).bold()),
        ]));

        for line in msg.content.lines() {
            lines.push(Line::from(format!("  {line}")));
        }

        lines.push(Line::from(""));
    }

    if !app.current_stream.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("nyzhi ", Style::default().fg(Color::Green).bold()),
        ]));
        for line in app.current_stream.lines() {
            lines.push(Line::from(format!("  {line}")));
        }
    }

    let chat = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0));

    frame.render_widget(chat, area);
}

fn draw_input(frame: &mut Frame, area: Rect, app: &App) {
    let prompt = match app.mode {
        AppMode::Input => "> ",
        AppMode::Streaming => "  streaming...",
    };

    let input = Paragraph::new(format!("{prompt}{}", app.input))
        .block(
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
