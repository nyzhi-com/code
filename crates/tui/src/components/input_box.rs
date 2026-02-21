use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, AppMode};
use crate::spinner::SpinnerState;
use crate::theme::Theme;

fn cursor_2d(input: &str, byte_pos: usize) -> (u16, u16) {
    let before = &input[..byte_pos.min(input.len())];
    let row = before.matches('\n').count() as u16;
    let last_nl = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = before[last_nl..].len() as u16;
    (row, col)
}

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme, spinner: &SpinnerState) {
    let border_color = match app.mode {
        AppMode::Input => theme.border_strong,
        _ => theme.border_default,
    };

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg_page));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match app.mode {
        AppMode::Streaming => {
            let content = Line::from(vec![
                Span::styled(
                    format!("{} ", spinner.current_frame()),
                    Style::default().fg(theme.accent),
                ),
                Span::styled("thinking...", Style::default().fg(theme.text_tertiary)),
            ]);
            frame.render_widget(
                Paragraph::new(content).style(Style::default().bg(theme.bg_page)),
                inner,
            );
        }
        AppMode::AwaitingApproval => {
            let content = Line::from(vec![
                Span::styled("[y/n] ", Style::default().fg(theme.accent).bold()),
                Span::styled("approve?", Style::default().fg(theme.text_secondary)),
            ]);
            frame.render_widget(
                Paragraph::new(content).style(Style::default().bg(theme.bg_page)),
                inner,
            );
        }
        AppMode::Input => {
            let prompt = "> ";
            let cont = "  ";
            let lines: Vec<Line> = app
                .input
                .split('\n')
                .enumerate()
                .map(|(i, line_text)| {
                    let prefix = if i == 0 { prompt } else { cont };
                    Line::from(vec![
                        Span::styled(prefix, Style::default().fg(theme.accent).bold()),
                        Span::styled(line_text, Style::default().fg(theme.text_primary)),
                    ])
                })
                .collect();

            let (cursor_row, cursor_col) = cursor_2d(&app.input, app.cursor_pos);
            let visible_height = inner.height;

            let scroll_offset = if visible_height == 0 {
                0
            } else if cursor_row >= visible_height {
                cursor_row - visible_height + 1
            } else {
                0
            };

            let paragraph = Paragraph::new(lines)
                .style(Style::default().bg(theme.bg_page))
                .scroll((scroll_offset, 0));
            frame.render_widget(paragraph, inner);

            let prefix_len = 2u16; // "> " or "  "
            frame.set_cursor_position(Position::new(
                inner.x + prefix_len + cursor_col,
                inner.y + cursor_row - scroll_offset,
            ));
        }
    }
}
