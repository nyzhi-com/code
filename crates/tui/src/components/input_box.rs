use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, AppMode};
use crate::completion::{CompletionContext, CompletionState};
use crate::spinner::SpinnerState;
use crate::theme::Theme;

fn cursor_2d(input: &str, byte_pos: usize) -> (u16, u16) {
    let before = &input[..byte_pos.min(input.len())];
    let row = before.matches('\n').count() as u16;
    let last_nl = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = before[last_nl..].len() as u16;
    (row, col)
}

const PROMPT_CHAR: &str = "❯";
const CONT_CHAR: &str = "·";

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme, spinner: &SpinnerState) {
    let focused = matches!(app.mode, AppMode::Input);
    let border_color = if focused {
        theme.accent
    } else {
        theme.border_default
    };

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg_surface));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let badge = mode_badge(app, theme);
    let badge_width = badge.width() as u16 + 1;

    let content_area = Rect::new(
        inner.x + badge_width,
        inner.y,
        inner.width.saturating_sub(badge_width),
        inner.height,
    );

    let badge_area = Rect::new(inner.x, inner.y, badge_width, 1);
    frame.render_widget(
        Paragraph::new(badge).style(Style::default().bg(theme.bg_surface)),
        badge_area,
    );

    match app.mode {
        AppMode::Streaming => render_streaming(frame, content_area, app, theme, spinner),
        AppMode::AwaitingApproval => render_approval(frame, content_area, theme),
        AppMode::AwaitingUserQuestion => render_question(frame, content_area, theme),
        AppMode::Input => {
            if let Some(search) = &app.history_search {
                render_history_search(frame, content_area, app, theme, search);
            } else {
                render_normal_input(frame, content_area, app, theme);
            }
        }
    }

    if let Some(state) = &app.completion {
        render_completion_popup(frame, area, state, theme);
    }
}

fn mode_badge<'a>(app: &App, theme: &Theme) -> Line<'a> {
    if app.plan_mode {
        Line::from(vec![
            Span::styled(
                " Plan ",
                Style::default().fg(theme.bg_page).bg(theme.warning).bold(),
            ),
            Span::raw(" "),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                " Act ",
                Style::default()
                    .fg(theme.text_tertiary)
                    .bg(theme.bg_elevated),
            ),
            Span::raw(" "),
        ])
    }
}

fn render_streaming(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    theme: &Theme,
    spinner: &SpinnerState,
) {
    if !app.input.is_empty() {
        let lines: Vec<Line> = app
            .input
            .split('\n')
            .enumerate()
            .map(|(i, line_text)| {
                let prefix = if i == 0 { PROMPT_CHAR } else { CONT_CHAR };
                Line::from(vec![
                    Span::styled(
                        format!("{prefix} "),
                        Style::default().fg(theme.warning).bold(),
                    ),
                    Span::styled(line_text, Style::default().fg(theme.text_primary)),
                ])
            })
            .collect();
        let paragraph = Paragraph::new(lines).style(Style::default().bg(theme.bg_surface));
        frame.render_widget(paragraph, area);

        let queue_hint = format!(" queued:{} ", app.message_queue.len() + 1);
        let hint_w = queue_hint.len() as u16;
        if area.width > hint_w + 2 {
            let hint_area = Rect::new(
                area.x + area.width - hint_w,
                area.y,
                hint_w,
                1,
            );
            frame.render_widget(
                Paragraph::new(Span::styled(
                    queue_hint,
                    Style::default()
                        .fg(theme.text_disabled)
                        .bg(theme.bg_elevated),
                )),
                hint_area,
            );
        }

        let (cursor_row, cursor_col) = cursor_2d(&app.input, app.cursor_pos);
        frame.set_cursor_position(Position::new(
            area.x + 2 + cursor_col,
            area.y + cursor_row,
        ));
    } else {
        let mut spans: Vec<Span> = vec![
            Span::styled(
                format!("{} ", spinner.current_frame()),
                Style::default().fg(theme.accent),
            ),
            Span::styled("thinking", Style::default().fg(theme.text_tertiary)),
        ];

        if let Some(start) = &app.turn_start {
            let ms = start.elapsed().as_millis() as u64;
            let elapsed = if ms < 1000 {
                format!("{ms}ms")
            } else if ms < 60_000 {
                format!("{:.1}s", ms as f64 / 1000.0)
            } else {
                let m = ms / 60_000;
                let s = (ms % 60_000) / 1000;
                format!("{m}m{s}s")
            };
            spans.push(Span::styled(
                format!(" {elapsed}"),
                Style::default().fg(theme.text_disabled),
            ));
        }

        let queue_count = app.message_queue.len();
        if queue_count > 0 {
            spans.push(Span::styled(
                format!("  queue:{queue_count}"),
                Style::default().fg(theme.text_disabled),
            ));
        }

        spans.push(Span::styled(
            "  type to queue",
            Style::default().fg(theme.text_disabled),
        ));

        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.bg_surface)),
            area,
        );
    }
}

fn render_approval(frame: &mut Frame, area: Rect, theme: &Theme) {
    let content = Line::from(vec![
        Span::styled(
            format!("{PROMPT_CHAR} "),
            Style::default().fg(theme.warning).bold(),
        ),
        Span::styled(
            "approve? ",
            Style::default().fg(theme.text_primary),
        ),
        Span::styled("y", Style::default().fg(theme.success).bold()),
        Span::styled("/", Style::default().fg(theme.text_disabled)),
        Span::styled("n", Style::default().fg(theme.danger).bold()),
    ]);
    frame.render_widget(
        Paragraph::new(content).style(Style::default().bg(theme.bg_surface)),
        area,
    );
}

fn render_question(frame: &mut Frame, area: Rect, theme: &Theme) {
    let content = Line::from(vec![
        Span::styled("? ", Style::default().fg(theme.info).bold()),
        Span::styled(
            "select an option above",
            Style::default().fg(theme.text_secondary),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(content).style(Style::default().bg(theme.bg_surface)),
        area,
    );
}

fn render_normal_input(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let lines: Vec<Line> = if app.input.is_empty() {
        vec![Line::from(vec![
            Span::styled(
                format!("{PROMPT_CHAR} "),
                Style::default().fg(theme.accent).bold(),
            ),
            Span::styled(
                "Ask anything... ",
                Style::default().fg(theme.text_disabled),
            ),
            Span::styled(
                "/",
                Style::default().fg(theme.text_disabled).bold(),
            ),
            Span::styled(
                " commands",
                Style::default().fg(theme.text_disabled),
            ),
        ])]
    } else {
        app.input
            .split('\n')
            .enumerate()
            .map(|(i, line_text)| {
                let prefix = if i == 0 { PROMPT_CHAR } else { CONT_CHAR };
                Line::from(vec![
                    Span::styled(
                        format!("{prefix} "),
                        Style::default().fg(theme.accent).bold(),
                    ),
                    Span::styled(line_text, Style::default().fg(theme.text_primary)),
                ])
            })
            .collect()
    };

    let (cursor_row, cursor_col) = cursor_2d(&app.input, app.cursor_pos);
    let visible_height = area.height;

    let scroll_offset = if visible_height == 0 {
        0
    } else if cursor_row >= visible_height {
        cursor_row - visible_height + 1
    } else {
        0
    };

    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(theme.bg_surface))
        .scroll((scroll_offset, 0));
    frame.render_widget(paragraph, area);

    frame.set_cursor_position(Position::new(
        area.x + 2 + cursor_col,
        area.y + cursor_row - scroll_offset,
    ));
}

fn render_completion_popup(
    frame: &mut Frame,
    input_area: Rect,
    state: &CompletionState,
    theme: &Theme,
) {
    let max_visible = state.max_visible();
    let count = state.candidates.len();
    let visible = count.min(max_visible);
    let popup_height = visible as u16 + 2;

    if input_area.y < popup_height {
        return;
    }

    let has_descriptions = state.context == CompletionContext::SlashCommand
        && state.descriptions.iter().any(|d| !d.is_empty());

    let max_name_width = state.candidates.iter().map(|c| c.len()).max().unwrap_or(10);

    let popup_width = if has_descriptions {
        let max_desc_width = state
            .descriptions
            .iter()
            .map(|d| d.len())
            .max()
            .unwrap_or(0);
        let total = max_name_width + 2 + max_desc_width + 4;
        (total as u16).min(input_area.width).max(30)
    } else {
        (max_name_width as u16 + 4).min(input_area.width).max(16)
    };

    let popup_area = Rect {
        x: input_area.x + 1,
        y: input_area.y - popup_height,
        width: popup_width,
        height: popup_height,
    };

    let title = match state.context {
        CompletionContext::SlashCommand => "Commands",
        CompletionContext::AtMention | CompletionContext::FilePath => "Files",
    };

    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_strong))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(theme.text_secondary),
        ))
        .style(Style::default().bg(theme.bg_elevated));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let name_col_width = if has_descriptions {
        max_name_width + 2
    } else {
        inner.width as usize
    };

    let visible_candidates: Vec<Line> = state
        .candidates
        .iter()
        .enumerate()
        .skip(state.scroll_offset)
        .take(max_visible)
        .map(|(i, candidate)| {
            let desc = state.descriptions.get(i).map(|d| d.as_str()).unwrap_or("");

            if has_descriptions {
                let name_display = if candidate.len() > name_col_width {
                    format!("{}...", &candidate[..name_col_width.saturating_sub(3)])
                } else {
                    format!("{:<width$}", candidate, width = name_col_width)
                };

                let remaining = (inner.width as usize).saturating_sub(name_col_width);
                let desc_display = if desc.len() > remaining {
                    format!("{}...", &desc[..remaining.saturating_sub(3)])
                } else {
                    format!("{:<width$}", desc, width = remaining)
                };

                if i == state.selected {
                    Line::from(vec![
                        Span::styled(
                            name_display,
                            Style::default().fg(theme.bg_page).bg(theme.accent).bold(),
                        ),
                        Span::styled(
                            desc_display,
                            Style::default().fg(theme.bg_elevated).bg(theme.accent),
                        ),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(name_display, Style::default().fg(theme.accent)),
                        Span::styled(desc_display, Style::default().fg(theme.text_tertiary)),
                    ])
                }
            } else {
                let display = if candidate.len() as u16 > inner.width {
                    format!(
                        "{}...",
                        &candidate[..(inner.width as usize).saturating_sub(3)]
                    )
                } else {
                    candidate.clone()
                };

                if i == state.selected {
                    Line::from(Span::styled(
                        format!("{display:<width$}", width = inner.width as usize),
                        Style::default().fg(theme.bg_page).bg(theme.accent),
                    ))
                } else {
                    Line::from(Span::styled(
                        display,
                        Style::default().fg(theme.text_primary),
                    ))
                }
            }
        })
        .collect();

    let paragraph =
        Paragraph::new(visible_candidates).style(Style::default().bg(theme.bg_elevated));
    frame.render_widget(paragraph, inner);
}

fn render_history_search(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    theme: &Theme,
    search: &crate::history::HistorySearch,
) {
    let matches = app.history.search(&search.query);
    let matched_entry = matches.get(search.selected).map(|(_, e)| *e);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            "search: ",
            Style::default().fg(theme.accent).bold(),
        ),
        Span::styled(&search.query, Style::default().fg(theme.text_primary)),
    ]));

    if let Some(entry) = matched_entry {
        let display = entry.replace('\n', " \\n ");
        let truncated = if display.len() > 200 {
            format!("{}...", &display[..200])
        } else {
            display
        };
        lines.push(Line::from(Span::styled(
            truncated,
            Style::default().fg(theme.text_secondary),
        )));
        if matches.len() > 1 {
            lines.push(Line::from(Span::styled(
                format!("  [{}/{}]", search.selected + 1, matches.len()),
                Style::default().fg(theme.text_tertiary),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "(no match)",
            Style::default().fg(theme.text_tertiary),
        )));
    }

    let paragraph = Paragraph::new(lines).style(Style::default().bg(theme.bg_surface));
    frame.render_widget(paragraph, area);

    let cursor_col = "search: ".len() as u16 + search.query.len() as u16;
    frame.set_cursor_position(Position::new(area.x + cursor_col, area.y));
}
