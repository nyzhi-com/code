use ratatui::prelude::*;

use crate::app::App;
use crate::components::{chat, footer, input_box, selector, welcome};
use crate::spinner::SpinnerState;
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, app: &App, theme: &Theme, spinner: &SpinnerState) {
    frame.render_widget(
        ratatui::widgets::Block::default().style(Style::default().bg(theme.bg_page)),
        frame.area(),
    );

    let input_lines = if app.history_search.is_some() {
        3u16 // search prompt + match + counter
    } else {
        app.input.lines().count().max(1) as u16
    };
    let input_height = (input_lines + 2).min(12); // +2 for borders, cap at 12

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(input_height),
            Constraint::Length(1),
        ])
        .split(frame.area());

    if app.items.is_empty() && app.current_stream.is_empty() {
        welcome::draw(frame, chunks[0], app, theme);
    } else {
        chat::draw(frame, chunks[0], app, theme);
    }

    input_box::draw(frame, chunks[1], app, theme, spinner);
    footer::draw(frame, chunks[2], app, theme);

    if let Some(sel) = &app.selector {
        selector::draw(frame, sel, theme);
    }
}
