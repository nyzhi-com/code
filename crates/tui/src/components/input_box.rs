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
            let mut spans = vec![
                Span::styled(
                    format!("{} ", spinner.current_frame()),
                    Style::default().fg(theme.accent),
                ),
                Span::styled("thinking...", Style::default().fg(theme.text_tertiary)),
            ];
            if let Some(start) = &app.turn_start {
                let elapsed_ms = start.elapsed().as_millis() as u64;
                let elapsed_str = if elapsed_ms < 1000 {
                    format!("{elapsed_ms}ms")
                } else if elapsed_ms < 60_000 {
                    format!("{:.1}s", elapsed_ms as f64 / 1000.0)
                } else {
                    let m = elapsed_ms / 60_000;
                    let s = (elapsed_ms % 60_000) / 1000;
                    format!("{m}m{s}s")
                };
                spans.push(Span::styled(
                    format!(" ({elapsed_str})"),
                    Style::default().fg(theme.text_tertiary),
                ));
            }
            let content = Line::from(spans);
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
        AppMode::AwaitingUserQuestion => {
            let content = Line::from(vec![
                Span::styled("? ", Style::default().fg(theme.accent).bold()),
                Span::styled("select an option above", Style::default().fg(theme.text_secondary)),
            ]);
            frame.render_widget(
                Paragraph::new(content).style(Style::default().bg(theme.bg_page)),
                inner,
            );
        }
        AppMode::Input => {
            if let Some(search) = &app.history_search {
                render_history_search(frame, inner, app, theme, search);
            } else {
                render_normal_input(frame, inner, app, theme);
            }
        }
    }

    if let Some(state) = &app.completion {
        render_completion_popup(frame, area, state, theme);
    }
}

fn render_normal_input(frame: &mut Frame, inner: Rect, app: &App, theme: &Theme) {
    let prompt = "> ";
    let cont = "  ";
    let lines: Vec<Line> = if app.input.is_empty() {
        vec![Line::from(vec![
            Span::styled(prompt, Style::default().fg(theme.accent).bold()),
            Span::styled("Ask anything, Ctrl+K for commands", Style::default().fg(theme.text_disabled)),
        ])]
    } else {
        app.input
            .split('\n')
            .enumerate()
            .map(|(i, line_text)| {
                let prefix = if i == 0 { prompt } else { cont };
                Line::from(vec![
                    Span::styled(prefix, Style::default().fg(theme.accent).bold()),
                    Span::styled(line_text, Style::default().fg(theme.text_primary)),
                ])
            })
            .collect()
    };

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

    let prefix_len = 2u16;
    frame.set_cursor_position(Position::new(
        inner.x + prefix_len + cursor_col,
        inner.y + cursor_row - scroll_offset,
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

    let max_name_width = state
        .candidates
        .iter()
        .map(|c| c.len())
        .max()
        .unwrap_or(10);

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
                    format!(
                        "{}...",
                        &candidate[..name_col_width.saturating_sub(3)]
                    )
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
                        Span::styled(
                            name_display,
                            Style::default().fg(theme.accent),
                        ),
                        Span::styled(
                            desc_display,
                            Style::default().fg(theme.text_tertiary),
                        ),
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
    inner: Rect,
    app: &App,
    theme: &Theme,
    search: &crate::history::HistorySearch,
) {
    let matches = app.history.search(&search.query);
    let matched_entry = matches.get(search.selected).map(|(_, e)| *e);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("(reverse-search): ", Style::default().fg(theme.accent).bold()),
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

    let paragraph = Paragraph::new(lines).style(Style::default().bg(theme.bg_page));
    frame.render_widget(paragraph, inner);

    let cursor_col = "(reverse-search): ".len() as u16 + search.query.len() as u16;
    frame.set_cursor_position(Position::new(inner.x + cursor_col, inner.y));
}
