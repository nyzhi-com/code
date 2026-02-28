use ratatui::prelude::*;

use crate::aesthetic::layout as aes_layout;
use crate::app::App;
use crate::components::{
    chat, footer, header, input_box, plan_banner, plan_panel, selector, settings_panel,
    text_prompt, todo_panel, update_banner, welcome,
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
    let banner_h = update_banner::height(&app.update_status);
    let plan_h = plan_banner::height(app.plan_mode);
    let has_content = !app.items.is_empty() || !app.current_stream.is_empty();

    let regions = aes_layout::compute(
        frame.area(),
        banner_h,
        plan_h,
        has_content,
        input_lines,
    );

    if banner_h > 0 {
        update_banner::draw(frame, regions.banner, &app.update_status, theme);
    }

    if plan_h > 0 {
        plan_banner::draw(frame, regions.plan_banner, theme);
    }

    if has_content {
        header::draw(frame, regions.header, app, theme);
    }

    let (chat_area, panel_area) = if app.show_plan_panel {
        aes_layout::split_side_panel(regions.main)
    } else {
        (regions.main, None)
    };

    if app.items.is_empty() && app.current_stream.is_empty() {
        welcome::draw(frame, chat_area, app, theme);
    } else {
        chat::draw(frame, chat_area, app, theme);
    }

    if let Some(panel_rect) = panel_area {
        plan_panel::draw(frame, panel_rect, &app.plan_panel, theme);
    }

    input_box::draw(frame, regions.input, app, theme, spinner);
    footer::draw(frame, regions.footer, app, theme);

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
