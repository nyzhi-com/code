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
    if area.height < 2 || area.width == 0 {
        return;
    }

    let status_h = 1u16;
    let input_h = area.height.saturating_sub(status_h);

    let input_area = Rect::new(area.x, area.y, area.width, input_h);
    let status_area = Rect::new(area.x, area.y + input_h, area.width, status_h);

    frame.render_widget(
        Block::default().style(Style::default().bg(theme.bg_page)),
        input_area,
    );

    match app.mode {
        AppMode::Streaming => render_streaming(frame, input_area, app, theme, spinner),
        AppMode::AwaitingApproval => render_approval(frame, input_area, app, theme),
        AppMode::AwaitingUserQuestion => render_question(frame, input_area, theme),
        AppMode::Input => {
            if let Some(search) = &app.history_search {
                render_history_search(frame, input_area, app, theme, search);
            } else {
                render_input(frame, input_area, app, theme);
            }
        }
    }

    render_status_bar(frame, status_area, app, theme);

    if let Some(state) = &app.completion {
        render_completion_popup(frame, area, state, theme);
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let mode_label = if app.plan_mode { "Plan" } else { "Build" };
    let mode_color = if app.plan_mode {
        theme.warning
    } else {
        theme.accent
    };

    let auth = nyzhi_auth::auth_status(&app.provider_name);
    let (model_text, provider_text) = if auth == "not connected" {
        ("not connected".to_string(), String::new())
    } else {
        (app.model_name.clone(), app.provider_name.clone())
    };
    let model_color = if auth == "not connected" {
        theme.text_disabled
    } else {
        theme.text_primary
    };

    let mut spans: Vec<Span> = vec![
        Span::styled(
            format!(" {mode_label}"),
            Style::default().fg(mode_color).bold(),
        ),
        Span::styled("  ", Style::default()),
        Span::styled(model_text, Style::default().fg(model_color)),
    ];

    if !provider_text.is_empty() {
        spans.push(Span::styled(
            format!("  {provider_text}"),
            Style::default().fg(theme.text_disabled),
        ));
    }

    if matches!(app.trust_mode, nyzhi_config::TrustMode::Full) {
        spans.push(Span::styled(
            "  YOLO",
            Style::default().fg(theme.danger).bold(),
        ));
    }

    let paragraph = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(theme.bg_surface));
    frame.render_widget(paragraph, area);
}

fn render_input(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    if app.input.is_empty() {
        frame.set_cursor_position(Position::new(area.x + 1, area.y));
        return;
    }

    let lines: Vec<Line> = app
        .input
        .split('\n')
        .map(|text| {
            Line::from(Span::styled(
                format!(" {text}"),
                Style::default().fg(theme.text_primary),
            ))
        })
        .collect();

    let (cursor_row, cursor_col) = cursor_2d(&app.input, app.cursor_pos);
    let visible_height = area.height;
    let scroll = if visible_height > 0 && cursor_row >= visible_height {
        cursor_row - visible_height + 1
    } else {
        0
    };

    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(theme.bg_page))
        .scroll((scroll, 0));
    frame.render_widget(paragraph, area);

    frame.set_cursor_position(Position::new(
        area.x + 1 + cursor_col,
        area.y + cursor_row - scroll,
    ));
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
            .map(|text| {
                Line::from(Span::styled(
                    format!(" {text}"),
                    Style::default().fg(theme.text_primary),
                ))
            })
            .collect();
        let paragraph = Paragraph::new(lines).style(Style::default().bg(theme.bg_page));
        frame.render_widget(paragraph, area);

        let queue_hint = format!(" queued:{} ", app.message_queue.len() + 1);
        let hint_w = queue_hint.len() as u16;
        if area.width > hint_w + 2 {
            let hint_area = Rect::new(area.x + area.width - hint_w, area.y, hint_w, 1);
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
            area.x + 1 + cursor_col,
            area.y + cursor_row,
        ));
    } else {
        let mut spans: Vec<Span> = vec![
            Span::styled(
                format!(" {} ", spinner.current_frame()),
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
            Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.bg_page)),
            area,
        );
    }
}

fn render_approval(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let buttons: [(&str, usize); 3] = [(" Allow ", 0), (" Deny ", 1), (" Always ", 2)];
    let btn_total_width: usize = buttons.iter().map(|(l, _)| l.len() + 2).sum::<usize>() + 2;

    let mut spans: Vec<Span> = Vec::new();

    if let Some((ref tool, ref args)) = app.pending_approval_context {
        spans.push(Span::styled(
            " ? ",
            Style::default().fg(theme.warning).bold(),
        ));
        spans.push(Span::styled(
            tool.clone(),
            Style::default().fg(theme.accent).bold(),
        ));

        let first_line = args.lines().next().unwrap_or("");
        let max_args = (area.width as usize).saturating_sub(tool.len() + btn_total_width + 6);
        if !first_line.is_empty() && max_args > 4 {
            let truncated = if first_line.len() > max_args {
                format!(" {}...", &first_line[..max_args.saturating_sub(3)])
            } else {
                format!(" {first_line}")
            };
            spans.push(Span::styled(
                truncated,
                Style::default().fg(theme.text_tertiary),
            ));
        }
    } else {
        spans.push(Span::styled(
            " ? ",
            Style::default().fg(theme.warning).bold(),
        ));
        spans.push(Span::styled(
            "approve? ",
            Style::default().fg(theme.text_primary),
        ));
    }

    let used: usize = spans.iter().map(|s| s.width()).sum();
    let gap = (area.width as usize).saturating_sub(used + btn_total_width);
    if gap > 0 {
        spans.push(Span::raw(" ".repeat(gap)));
    }

    for (label, idx) in &buttons {
        if *idx == app.approval_cursor {
            spans.push(Span::styled(
                format!("[{label}]"),
                Style::default().fg(theme.bg_page).bg(theme.accent).bold(),
            ));
        } else {
            spans.push(Span::styled(
                format!("[{label}]"),
                Style::default()
                    .fg(theme.text_secondary)
                    .bg(theme.bg_elevated),
            ));
        }
        spans.push(Span::raw(" "));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.bg_page)),
        area,
    );
}

fn render_question(frame: &mut Frame, area: Rect, theme: &Theme) {
    let content = Line::from(vec![
        Span::styled(" ? ", Style::default().fg(theme.info).bold()),
        Span::styled(
            "select an option above",
            Style::default().fg(theme.text_secondary),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(content).style(Style::default().bg(theme.bg_page)),
        area,
    );
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
        Span::styled(" search: ", Style::default().fg(theme.accent).bold()),
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
            format!(" {truncated}"),
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
            " (no match)",
            Style::default().fg(theme.text_tertiary),
        )));
    }

    let paragraph = Paragraph::new(lines).style(Style::default().bg(theme.bg_page));
    frame.render_widget(paragraph, area);

    let cursor_col = " search: ".len() as u16 + search.query.len() as u16;
    frame.set_cursor_position(Position::new(area.x + cursor_col, area.y));
}
