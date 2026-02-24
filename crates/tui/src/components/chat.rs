use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, DiffLineKind, DisplayItem, ToolStatus};
use crate::highlight::{self, SyntaxHighlighter};
use crate::theme::{Theme, ThemeMode};

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_default))
        .title(Line::from(vec![Span::styled(
            " nyzhi code ",
            Style::default().fg(theme.accent).bold(),
        )]))
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg_page));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let dark = theme.mode == ThemeMode::Dark;
    let mut lines: Vec<Line> = Vec::new();

    let search_q = app.search_query.as_deref();
    let current_match_item = if !app.search_matches.is_empty() {
        app.search_matches.get(app.search_match_idx).copied()
    } else {
        None
    };

    for (item_idx, item) in app.items.iter().enumerate() {
        let is_match = search_q.is_some() && app.search_matches.contains(&item_idx);
        let is_current = current_match_item == Some(item_idx);
        let line_start = lines.len();

        match item {
            DisplayItem::Message { role, content } => {
                render_message(&mut lines, role, content, theme, &app.highlighter, dark);
            }
            DisplayItem::Thinking(content) => {
                render_thinking(&mut lines, content, theme, false);
            }
            DisplayItem::ToolCall {
                name,
                args_summary,
                output,
                status,
                elapsed_ms,
            } => {
                let tool_start = lines.len();
                match app.output_style {
                    nyzhi_config::OutputStyle::Minimal => {
                        render_tool_minimal(&mut lines, name, status, theme);
                    }
                    _ => {
                        render_tool_call(
                            &mut lines,
                            name,
                            args_summary,
                            output,
                            status,
                            elapsed_ms,
                            theme,
                        );
                    }
                }
                prepend_bar(&mut lines[tool_start..], theme.text_disabled);
            }
            DisplayItem::Diff {
                file,
                hunks,
                is_new_file,
            } => {
                render_diff(&mut lines, file, hunks, *is_new_file, theme);
            }
        }

        if is_match {
            if let Some(q) = search_q {
                let hl_style = if is_current {
                    Style::default().bg(Color::Yellow).fg(Color::Black)
                } else {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                };
                for line in &mut lines[line_start..] {
                    *line = highlight_search_in_line(line.clone(), q, hl_style);
                }
            }
        }
    }

    if !app.thinking_stream.is_empty() && app.current_stream.is_empty() {
        lines.push(Line::from(""));
        render_thinking_stream(&mut lines, &app.thinking_stream, theme);
    }

    if !app.current_stream.is_empty() {
        lines.push(Line::from(""));
        let stream_start = lines.len();
        render_highlighted_content(
            &mut lines,
            &app.current_stream,
            theme,
            &app.highlighter,
            dark,
        );
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("█", Style::default().fg(theme.accent)),
        ]));
        prepend_bar(&mut lines[stream_start..], theme.accent);
    }

    let total_lines = lines.len() as u16;
    let visible = inner.height;
    let auto_scroll = total_lines.saturating_sub(visible);

    let scroll = if app.scroll_offset > 0 {
        auto_scroll.saturating_sub(app.scroll_offset)
    } else {
        auto_scroll
    };

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0))
        .style(Style::default().bg(theme.bg_page));

    frame.render_widget(paragraph, inner);
}

fn highlight_search_in_line<'a>(line: Line<'a>, query: &str, hl_style: Style) -> Line<'a> {
    let query_lower = query.to_lowercase();
    let mut new_spans: Vec<Span<'a>> = Vec::new();

    for span in line.spans {
        let text = span.content.to_string();
        let text_lower = text.to_lowercase();
        let base_style = span.style;

        let mut start = 0;
        let mut found = false;
        while let Some(pos) = text_lower[start..].find(&query_lower) {
            found = true;
            let abs_pos = start + pos;
            if abs_pos > start {
                new_spans.push(Span::styled(text[start..abs_pos].to_string(), base_style));
            }
            new_spans.push(Span::styled(
                text[abs_pos..abs_pos + query.len()].to_string(),
                hl_style,
            ));
            start = abs_pos + query.len();
        }
        if found {
            if start < text.len() {
                new_spans.push(Span::styled(text[start..].to_string(), base_style));
            }
        } else {
            new_spans.push(Span::styled(text, base_style));
        }
    }

    Line::from(new_spans)
}

fn prepend_bar(lines: &mut [Line<'_>], color: Color) {
    for line in lines.iter_mut() {
        line.spans
            .insert(0, Span::styled(" ┃ ", Style::default().fg(color)));
    }
}

fn role_label<'a>(role: &str, theme: &Theme) -> (Vec<Span<'a>>, Color) {
    match role {
        "user" => {
            let spans = vec![
                Span::styled("  You", Style::default().fg(theme.info).bold()),
            ];
            (spans, theme.info)
        }
        "system" => {
            let spans = vec![
                Span::styled("  system", Style::default().fg(theme.text_disabled).italic()),
            ];
            (spans, theme.text_disabled)
        }
        _ => {
            let spans = vec![
                Span::styled("  Nizzy", Style::default().fg(theme.accent).bold()),
            ];
            (spans, theme.accent)
        }
    }
}

fn render_message<'a>(
    lines: &mut Vec<Line<'a>>,
    role: &str,
    content: &str,
    theme: &Theme,
    highlighter: &SyntaxHighlighter,
    dark: bool,
) {
    lines.push(Line::from(""));
    let bar_start = lines.len();

    let (label_spans, bar_color) = role_label(role, theme);
    lines.push(Line::from(label_spans));

    match role {
        "user" => {
            for line in content.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  {line}"),
                    Style::default().fg(theme.text_primary).bold(),
                )));
            }
        }
        "system" => {
            for line in content.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  {line}"),
                    Style::default().fg(theme.text_secondary).italic(),
                )));
            }
        }
        _ => {
            render_highlighted_content(lines, content, theme, highlighter, dark);
        }
    }

    prepend_bar(&mut lines[bar_start..], bar_color);
}

fn render_thinking<'a>(lines: &mut Vec<Line<'a>>, content: &str, theme: &Theme, _streaming: bool) {
    lines.push(Line::from(""));
    let think_start = lines.len();
    let dim = Style::default()
        .fg(theme.text_disabled)
        .add_modifier(Modifier::ITALIC);

    lines.push(Line::from(vec![
        Span::styled("  thinking ", dim),
        Span::styled(
            format!("({} lines)", content.lines().count()),
            Style::default().fg(theme.text_disabled),
        ),
    ]));

    for line_text in content.lines().take(8) {
        let trimmed = if line_text.len() > 120 {
            format!("{}...", &line_text[..117])
        } else {
            line_text.to_string()
        };
        lines.push(Line::from(Span::styled(format!("  {trimmed}"), dim)));
    }
    if content.lines().count() > 8 {
        lines.push(Line::from(Span::styled(
            format!("  ... +{} more", content.lines().count() - 8),
            Style::default().fg(theme.text_disabled),
        )));
    }
    prepend_bar(&mut lines[think_start..], theme.text_disabled);
}

fn render_thinking_stream<'a>(lines: &mut Vec<Line<'a>>, content: &str, theme: &Theme) {
    let think_start = lines.len();
    let dim = Style::default()
        .fg(theme.text_disabled)
        .add_modifier(Modifier::ITALIC);

    lines.push(Line::from(Span::styled("  thinking...", dim)));
    let tlines: Vec<&str> = content.lines().collect();
    let show = tlines.len().min(6);
    let start = if show < tlines.len() {
        tlines.len() - show
    } else {
        0
    };
    for line_text in &tlines[start..] {
        let trimmed = if line_text.len() > 120 {
            format!("{}...", &line_text[..117])
        } else {
            (*line_text).to_string()
        };
        lines.push(Line::from(Span::styled(format!("  {trimmed}"), dim)));
    }
    prepend_bar(&mut lines[think_start..], theme.text_disabled);
}

fn render_highlighted_content<'a>(
    lines: &mut Vec<Line<'a>>,
    content: &str,
    theme: &Theme,
    highlighter: &SyntaxHighlighter,
    dark: bool,
) {
    let segments = highlight::parse_segments(content);
    let code_bg = theme.bg_elevated;

    for segment in segments {
        match segment {
            highlight::Segment::Prose(text) => {
                for line in text.lines() {
                    lines.push(highlight::format_prose_line(
                        line,
                        theme.text_primary,
                        theme.accent,
                        code_bg,
                    ));
                }
            }
            highlight::Segment::CodeBlock { lang, code } => {
                let lang_label = lang.unwrap_or("text");
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!(" {lang_label} "),
                        Style::default()
                            .fg(theme.text_secondary)
                            .bg(theme.bg_elevated)
                            .bold(),
                    ),
                ]));

                let highlighted =
                    highlighter.highlight_code(code, lang, dark, theme.text_disabled, code_bg);
                for hl_line in highlighted {
                    let mut padded = vec![Span::raw("  ")];
                    padded.extend(hl_line.spans);
                    lines.push(Line::from(padded));
                }

                lines.push(Line::from(""));
            }
        }
    }
}

fn format_elapsed(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let m = ms / 60_000;
        let s = (ms % 60_000) / 1000;
        format!("{m}m{s}s")
    }
}

fn tool_icon(status: &ToolStatus) -> (&'static str, fn(&Theme) -> Color) {
    match status {
        ToolStatus::Running => ("◌", |t: &Theme| t.warning),
        ToolStatus::WaitingApproval => ("?", |t: &Theme| t.warning),
        ToolStatus::Completed => ("✓", |t: &Theme| t.success),
        ToolStatus::Denied => ("✗", |t: &Theme| t.danger),
    }
}

fn render_tool_minimal<'a>(
    lines: &mut Vec<Line<'a>>,
    name: &str,
    status: &ToolStatus,
    theme: &Theme,
) {
    let (icon, color_fn) = tool_icon(status);
    let color = color_fn(theme);
    lines.push(Line::from(vec![
        Span::styled(format!("    {icon} "), Style::default().fg(color)),
        Span::styled(
            name.to_string(),
            Style::default().fg(theme.text_disabled),
        ),
    ]));
}

fn render_tool_call<'a>(
    lines: &mut Vec<Line<'a>>,
    name: &str,
    args_summary: &str,
    output: &Option<String>,
    status: &ToolStatus,
    elapsed_ms: &Option<u64>,
    theme: &Theme,
) {
    let (icon, color_fn) = tool_icon(status);
    let icon_color = color_fn(theme);

    let mut summary_lines = args_summary.lines();
    let first_line = summary_lines.next().unwrap_or("");

    let summary = if first_line.len() > 80 {
        format!("{}...", &first_line[..77])
    } else {
        first_line.to_string()
    };

    let mut spans = vec![
        Span::styled(format!("    {icon} "), Style::default().fg(icon_color)),
        Span::styled(
            name.to_string(),
            Style::default().fg(theme.accent).bold(),
        ),
    ];

    if !summary.is_empty() {
        spans.push(Span::styled(
            format!(" {summary}"),
            Style::default().fg(theme.text_tertiary),
        ));
    }

    if *status == ToolStatus::Completed {
        if let Some(ms) = elapsed_ms {
            spans.push(Span::styled(
                format!(" {}", format_elapsed(*ms)),
                Style::default().fg(theme.text_disabled),
            ));
        }
    }

    lines.push(Line::from(spans));

    if *status == ToolStatus::WaitingApproval {
        for diff_line in summary_lines {
            lines.push(render_diff_line(diff_line, theme));
        }
    }

    if let Some(out) = output {
        let max_lines = if *status == ToolStatus::Running {
            10
        } else {
            3
        };
        let all_lines: Vec<&str> = out.lines().collect();
        let display_lines = if *status == ToolStatus::Running && all_lines.len() > max_lines {
            &all_lines[all_lines.len() - max_lines..]
        } else {
            &all_lines[..all_lines.len().min(max_lines)]
        };
        for line in display_lines {
            let truncated = if line.len() > 120 {
                format!("{}...", &line[..117])
            } else {
                (*line).to_string()
            };
            lines.push(Line::from(Span::styled(
                format!("      {truncated}"),
                Style::default().fg(theme.text_disabled),
            )));
        }
        if all_lines.len() > max_lines {
            lines.push(Line::from(Span::styled(
                format!("      ... +{} lines", all_lines.len() - max_lines),
                Style::default().fg(theme.text_disabled),
            )));
        }
    }
}

fn render_diff<'a>(
    lines: &mut Vec<Line<'a>>,
    file: &str,
    hunks: &[crate::app::DiffHunk],
    is_new_file: bool,
    theme: &Theme,
) {
    lines.push(Line::from(""));
    let diff_start = lines.len();

    let header_label = if is_new_file {
        format!("  + {file}")
    } else {
        format!("  ~ {file}")
    };
    let header_color = if is_new_file {
        theme.success
    } else {
        theme.accent
    };
    lines.push(Line::from(Span::styled(
        header_label,
        Style::default().fg(header_color).bold(),
    )));

    for hunk in hunks {
        lines.push(Line::from(Span::styled(
            format!("  {}", hunk.header),
            Style::default().fg(theme.info),
        )));
        for dl in &hunk.lines {
            let (prefix, color) = match dl.kind {
                DiffLineKind::Added => ("+", theme.success),
                DiffLineKind::Removed => ("-", theme.danger),
                DiffLineKind::Context => (" ", theme.text_disabled),
            };
            let line_text = if dl.content.len() > 120 {
                format!("  {prefix}{}", &dl.content[..117])
            } else {
                format!("  {prefix}{}", dl.content)
            };
            lines.push(Line::from(Span::styled(
                line_text,
                Style::default().fg(color),
            )));
        }
    }
    if hunks.is_empty() && !is_new_file {
        lines.push(Line::from(Span::styled(
            "  (no changes)",
            Style::default().fg(theme.text_disabled),
        )));
    }

    prepend_bar(&mut lines[diff_start..], if is_new_file { theme.success } else { theme.warning });
}

fn render_diff_line<'a>(line: &str, theme: &Theme) -> Line<'a> {
    let color = if line.starts_with('+') {
        theme.success
    } else if line.starts_with('-') {
        theme.danger
    } else if line.starts_with("@@") {
        theme.info
    } else {
        theme.text_tertiary
    };
    Line::from(Span::styled(
        format!("      {line}"),
        Style::default().fg(color),
    ))
}
