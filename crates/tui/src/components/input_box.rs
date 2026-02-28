use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::aesthetic::borders;
use crate::aesthetic::primitives;
use crate::aesthetic::tokens::*;
use crate::aesthetic::typography as ty;
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

fn border_color_for_mode(app: &App, theme: &Theme) -> Color {
    match app.mode {
        AppMode::Input => {
            if app.input.is_empty() && app.history_search.is_none() {
                theme.border_default
            } else {
                theme.accent
            }
        }
        AppMode::Streaming => theme.text_tertiary,
        AppMode::AwaitingApproval => theme.warning,
        AppMode::AwaitingUserQuestion => theme.info,
    }
}

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme, spinner: &SpinnerState) {
    if area.height < 3 || area.width < 6 {
        return;
    }

    let border_color = border_color_for_mode(app, theme);

    let show_title = matches!(app.mode, AppMode::Input) && app.history_search.is_none();
    let mut block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg_surface));

    if show_title {
        block = block.title(
            Line::from(Span::styled(
                " nyzhi ",
                Style::default().fg(border_color).bold(),
            ))
            .alignment(Alignment::Left),
        );
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let padded = Rect::new(
        inner.x + PAD_H,
        inner.y,
        inner.width.saturating_sub(PAD_H * 2),
        inner.height,
    );

    if padded.height < 2 || padded.width == 0 {
        return;
    }

    let sep_and_status_h: u16 = 2;
    let content_h = padded.height.saturating_sub(sep_and_status_h);

    let content_area = Rect::new(padded.x, padded.y, padded.width, content_h.max(1));
    let sep_area = Rect::new(
        padded.x,
        padded.y + content_h.max(1),
        padded.width,
        1,
    );
    let status_area = Rect::new(
        padded.x,
        padded.y + content_h.max(1) + 1,
        padded.width,
        1,
    );

    match app.mode {
        AppMode::Streaming => render_streaming(frame, content_area, app, theme, spinner),
        AppMode::AwaitingApproval => render_approval(frame, content_area, app, theme),
        AppMode::AwaitingUserQuestion => render_question(frame, content_area, theme),
        AppMode::Input => {
            if let Some(search) = &app.history_search {
                render_history_search(frame, content_area, app, theme, search);
            } else {
                render_input(frame, content_area, app, theme);
            }
        }
    }

    let sep_line = borders::THIN_H.repeat(padded.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(
            sep_line,
            Style::default().fg(theme.border_default),
        ))
        .style(Style::default().bg(theme.bg_surface)),
        sep_area,
    );

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
    let model_style = if auth == "not connected" {
        ty::disabled(theme)
    } else {
        ty::body(theme)
    };

    let mut spans: Vec<Span> = vec![
        Span::styled(mode_label.to_string(), Style::default().fg(mode_color).bold()),
        Span::raw("  "),
        Span::styled(model_text, model_style),
    ];

    if !provider_text.is_empty() {
        spans.push(Span::styled(
            format!("  {provider_text}"),
            ty::disabled(theme),
        ));
    }

    if matches!(app.trust_mode, nyzhi_config::TrustMode::Full) {
        spans.push(Span::styled("  YOLO", ty::danger(theme)));
    }

    let paragraph = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(theme.bg_surface));
    frame.render_widget(paragraph, area);
}

fn render_input(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    if app.input.is_empty() {
        let placeholder = "Ask anything... \"What is the tech stack of this project?\"";
        frame.render_widget(
            Paragraph::new(Span::styled(placeholder, ty::disabled(theme)))
                .style(Style::default().bg(theme.bg_surface)),
            area,
        );
        frame.set_cursor_position(Position::new(area.x, area.y));
        return;
    }

    let lines: Vec<Line> = app
        .input
        .split('\n')
        .map(|text| Line::from(Span::styled(text.to_string(), ty::body(theme))))
        .collect();

    let (cursor_row, cursor_col) = cursor_2d(&app.input, app.cursor_pos);
    let visible_height = area.height;
    let scroll = if visible_height > 0 && cursor_row >= visible_height {
        cursor_row - visible_height + 1
    } else {
        0
    };

    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(theme.bg_surface))
        .scroll((scroll, 0));
    frame.render_widget(paragraph, area);

    frame.set_cursor_position(Position::new(
        area.x + cursor_col,
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
            .map(|text| Line::from(Span::styled(text.to_string(), ty::body(theme))))
            .collect();
        let paragraph = Paragraph::new(lines)
            .style(Style::default().bg(theme.bg_surface));
        frame.render_widget(paragraph, area);

        let queue_hint = format!(" queued:{} ", app.message_queue.len() + 1);
        let hint_w = queue_hint.len() as u16;
        if area.width > hint_w + SP_2 {
            let hint_area = Rect::new(
                area.x + area.width - hint_w,
                area.y,
                hint_w,
                1,
            );
            frame.render_widget(
                Paragraph::new(Span::styled(
                    queue_hint,
                    ty::disabled(theme).bg(theme.bg_elevated),
                )),
                hint_area,
            );
        }

        let (cursor_row, cursor_col) = cursor_2d(&app.input, app.cursor_pos);
        frame.set_cursor_position(Position::new(
            area.x + cursor_col,
            area.y + cursor_row,
        ));
    } else {
        let mut spans: Vec<Span> = vec![
            Span::styled(
                format!("{} ", spinner.current_frame()),
                ty::accent(theme.accent),
            ),
            Span::styled("thinking", ty::caption(theme)),
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
            spans.push(Span::styled(format!(" {elapsed}"), ty::disabled(theme)));
        }

        let queue_count = app.message_queue.len();
        if queue_count > 0 {
            spans.push(Span::styled(
                format!("  queue:{queue_count}"),
                ty::disabled(theme),
            ));
        }

        spans.push(Span::styled("  type to queue", ty::disabled(theme)));

        frame.render_widget(
            Paragraph::new(Line::from(spans))
                .style(Style::default().bg(theme.bg_surface)),
            area,
        );
    }
}

fn render_approval(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let buttons: [(&str, usize); 3] = [(" Allow ", 0), (" Deny ", 1), (" Always ", 2)];
    let btn_total_width: usize = buttons.iter().map(|(l, _)| l.len() + 2).sum::<usize>() + 2;

    let mut spans: Vec<Span> = Vec::new();

    if let Some((ref tool, ref args)) = app.pending_approval_context {
        spans.push(Span::styled("? ", ty::warning_style(theme)));
        spans.push(Span::styled(tool.clone(), ty::subheading(theme)));

        let first_line = args.lines().next().unwrap_or("");
        let max_args = (area.width as usize).saturating_sub(tool.len() + btn_total_width + 6);
        if !first_line.is_empty() && max_args > 4 {
            let truncated = if first_line.len() > max_args {
                format!(" {}...", &first_line[..max_args.saturating_sub(3)])
            } else {
                format!(" {first_line}")
            };
            spans.push(Span::styled(truncated, ty::caption(theme)));
        }
    } else {
        spans.push(Span::styled("? ", ty::warning_style(theme)));
        spans.push(Span::styled("approve? ", ty::body(theme)));
    }

    let used: usize = spans.iter().map(|s| s.width()).sum();
    let gap = (area.width as usize).saturating_sub(used + btn_total_width);
    if gap > 0 {
        spans.push(Span::raw(" ".repeat(gap)));
    }

    for (label, idx) in &buttons {
        if *idx == app.approval_cursor {
            spans.push(primitives::pill(
                label.trim(),
                theme.bg_page,
                theme.accent,
            ));
        } else {
            spans.push(primitives::pill_outline(label.trim(), theme.text_secondary));
        }
        spans.push(Span::raw(" "));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .style(Style::default().bg(theme.bg_surface)),
        area,
    );
}

fn render_question(frame: &mut Frame, area: Rect, theme: &Theme) {
    let content = Line::from(vec![
        Span::styled("? ", ty::accent(theme.info)),
        Span::styled("select an option above", ty::secondary(theme)),
    ]);
    frame.render_widget(
        Paragraph::new(content)
            .style(Style::default().bg(theme.bg_surface)),
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

    let prefix_w: usize = 1;

    let popup_width = if has_descriptions {
        let max_desc_width = state
            .descriptions
            .iter()
            .map(|d| d.len())
            .max()
            .unwrap_or(0);
        let total = prefix_w + max_name_width + 2 + max_desc_width + PAD_H as usize * 2;
        (total as u16).min(input_area.width).max(POPUP_MIN_W)
    } else {
        (prefix_w as u16 + max_name_width as u16 + PAD_H * 2)
            .min(input_area.width)
            .max(16)
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

    let card = primitives::Card::new(theme)
        .title(title)
        .border(theme.accent);
    let inner = card.render_frame(frame, popup_area);

    let inner_w = inner.width as usize;
    let content_w = inner_w.saturating_sub(prefix_w);

    let name_col_width = if has_descriptions {
        max_name_width + SP_2 as usize
    } else {
        content_w
    };

    let visible_candidates: Vec<Line> = state
        .candidates
        .iter()
        .enumerate()
        .skip(state.scroll_offset)
        .take(max_visible)
        .map(|(i, candidate)| {
            let is_selected = i == state.selected;
            let row_bg = if is_selected { theme.accent } else { theme.bg_elevated };
            let primary_fg = if is_selected { theme.bg_page } else { theme.text_primary };
            let secondary_fg = if is_selected { theme.bg_elevated } else { theme.text_tertiary };
            let row_style = Style::default().fg(primary_fg).bg(row_bg);

            let desc = state.descriptions.get(i).map(|d| d.as_str()).unwrap_or("");

            let mut spans = vec![
                Span::styled(" ", Style::default().bg(row_bg)),
            ];

            if has_descriptions {
                let name_display = if candidate.len() > name_col_width {
                    format!("{}...", &candidate[..name_col_width.saturating_sub(3)])
                } else {
                    format!("{:<width$}", candidate, width = name_col_width)
                };

                let remaining = content_w.saturating_sub(name_col_width);
                let desc_display = if desc.len() > remaining {
                    format!("{}...", &desc[..remaining.saturating_sub(3)])
                } else {
                    format!("{:<width$}", desc, width = remaining)
                };

                spans.push(Span::styled(name_display, row_style.bold()));
                spans.push(Span::styled(
                    desc_display,
                    Style::default().fg(secondary_fg).bg(row_bg),
                ));
            } else {
                let display = if candidate.len() > content_w {
                    format!("{}...", &candidate[..content_w.saturating_sub(3)])
                } else {
                    candidate.clone()
                };
                spans.push(Span::styled(display.clone(), row_style.bold()));
                let used = prefix_w + display.len();
                let trail = inner_w.saturating_sub(used);
                if trail > 0 {
                    spans.push(Span::styled(
                        " ".repeat(trail),
                        Style::default().bg(row_bg),
                    ));
                }
            }

            Line::from(spans)
        })
        .collect();

    let paragraph = Paragraph::new(visible_candidates).style(ty::on_elevated(theme));
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
        Span::styled("search: ", ty::subheading(theme)),
        Span::styled(&search.query, ty::body(theme)),
    ]));

    if let Some(entry) = matched_entry {
        let display = entry.replace('\n', " \\n ");
        let truncated = if display.len() > 200 {
            format!("{}...", &display[..200])
        } else {
            display
        };
        lines.push(Line::from(Span::styled(truncated, ty::secondary(theme))));
        if matches.len() > 1 {
            lines.push(Line::from(Span::styled(
                format!(" [{}/{}]", search.selected + 1, matches.len()),
                ty::caption(theme),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(" (no match)", ty::caption(theme))));
    }

    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(theme.bg_surface));
    frame.render_widget(paragraph, area);

    let cursor_col = "search: ".len() as u16 + search.query.len() as u16;
    frame.set_cursor_position(Position::new(area.x + cursor_col, area.y));
}
