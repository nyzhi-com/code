use ratatui::prelude::*;

use crate::app::App;
use crate::components::{
    chat, footer, input_box, plan_banner, plan_panel, selector, settings_panel, text_prompt,
    todo_panel, update_banner, welcome,
};
use crate::spinner::SpinnerState;
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, app: &App, theme: &Theme, spinner: &SpinnerState) {
    frame.render_widget(
        ratatui::widgets::Block::default().style(Style::default().bg(theme.bg_page)),
        frame.area(),
    );

    let input_lines = if app.history_search.is_some() {
        3u16
    } else {
        app.input.lines().count().max(1) as u16
    };
    let input_height = (input_lines + 1).max(2).min(10);
    let banner_height = update_banner::height(&app.update_status);
    let plan_height = plan_banner::height(app.plan_mode);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(banner_height),
            Constraint::Length(plan_height),
            Constraint::Min(1),
            Constraint::Length(input_height),
            Constraint::Length(1),
        ])
        .split(frame.area());

    if banner_height > 0 {
        update_banner::draw(frame, chunks[0], &app.update_status, theme);
    }

    if plan_height > 0 {
        plan_banner::draw(frame, chunks[1], theme);
    }

    let main_area = chunks[2];

    let (chat_area, panel_area) = if app.show_plan_panel {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(main_area);
        (split[0], Some(split[1]))
    } else {
        (main_area, None)
    };

    if app.items.is_empty() && app.current_stream.is_empty() {
        welcome::draw(frame, chat_area, app, theme);
    } else {
        chat::draw(frame, chat_area, app, theme);
    }

    if let Some(panel_rect) = panel_area {
        plan_panel::draw(frame, panel_rect, &app.plan_panel, theme);
    }

    input_box::draw(frame, chunks[3], app, theme, spinner);
    footer::draw(frame, chunks[4], app, theme);

    if let Some(sel) = &app.selector {
        selector::draw(frame, sel, theme);
    }

    if let Some(prompt) = &app.text_prompt {
        text_prompt::draw(frame, prompt, theme);
    }

    if let Some(ref tp) = app.todo_panel {
        todo_panel::draw(frame, tp, theme);
    }

    if let Some(ref panel) = app.settings_panel {
        settings_panel::draw(frame, panel, theme);
    }
}
