use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, DisplayItem, ToolStatus};
use crate::highlight::{self, SyntaxHighlighter};
use crate::theme::{Theme, ThemeMode};

pub fn draw(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_default))
        .title(Line::from(vec![
            Span::styled(" nyzhi code ", Style::default().fg(theme.accent).bold()),
        ]))
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
        let is_match = search_q.is_some()
            && app.search_matches.contains(&item_idx);
        let is_current = current_match_item == Some(item_idx);
        let line_start = lines.len();

        match item {
            DisplayItem::Message { role, content } => {
                render_message(&mut lines, role, content, theme, &app.highlighter, dark);
            }
            DisplayItem::Thinking(content) => {
                let dim = Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC);
                lines.push(Line::from(Span::styled("  Thinking...", dim)));
                for line_text in content.lines().take(10) {
                    let trimmed = if line_text.len() > 120 {
                        format!("{}...", &line_text[..117])
                    } else {
                        line_text.to_string()
                    };
                    lines.push(Line::from(Span::styled(format!("  {trimmed}"), dim)));
                }
                if content.lines().count() > 10 {
                    lines.push(Line::from(Span::styled(
                        format!("  ... ({} more lines)", content.lines().count() - 10),
                        dim,
                    )));
                }
            }
            DisplayItem::ToolCall {
                name,
                args_summary,
                output,
                status,
                elapsed_ms,
            } => {
                render_tool_call(&mut lines, name, args_summary, output, status, elapsed_ms, theme);
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
        let dim = Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC);
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  Thinking...", dim)));
        let tlines: Vec<&str> = app.thinking_stream.lines().collect();
        let show = tlines.len().min(6);
        if show < tlines.len() {
            for line_text in &tlines[tlines.len() - show..] {
                let trimmed = if line_text.len() > 120 {
                    format!("{}...", &line_text[..117])
                } else {
                    (*line_text).to_string()
                };
                lines.push(Line::from(Span::styled(format!("  {trimmed}"), dim)));
            }
        } else {
            for line_text in &tlines {
                let trimmed = if line_text.len() > 120 {
                    format!("{}...", &line_text[..117])
                } else {
                    (*line_text).to_string()
                };
                lines.push(Line::from(Span::styled(format!("  {trimmed}"), dim)));
            }
        }
    }

    if !app.current_stream.is_empty() {
        lines.push(Line::from(""));
        render_highlighted_content(&mut lines, &app.current_stream, theme, &app.highlighter, dark);
        lines.push(Line::from(Span::styled(
            "  _",
            Style::default().fg(theme.accent),
        )));
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

fn render_message<'a>(
    lines: &mut Vec<Line<'a>>,
    role: &str,
    content: &str,
    theme: &Theme,
    highlighter: &SyntaxHighlighter,
    dark: bool,
) {
    lines.push(Line::from(""));

    match role {
        "user" => {
            for line in content.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  {line}"),
                    Style::default().fg(theme.text_primary).bold(),
                )));
            }
        }
        _ => {
            render_highlighted_content(lines, content, theme, highlighter, dark);
        }
    }
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

                let highlighted = highlighter.highlight_code(
                    code,
                    lang,
                    dark,
                    theme.text_disabled,
                    code_bg,
                );
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

fn render_tool_call<'a>(
    lines: &mut Vec<Line<'a>>,
    name: &str,
    args_summary: &str,
    output: &Option<String>,
    status: &ToolStatus,
    elapsed_ms: &Option<u64>,
    theme: &Theme,
) {
    let (icon, icon_color) = match status {
        ToolStatus::Running => ("*", theme.warning),
        ToolStatus::WaitingApproval => ("?", theme.accent),
        ToolStatus::Completed => ("+", theme.success),
        ToolStatus::Denied => ("x", theme.danger),
    };

    let mut summary_lines = args_summary.lines();
    let first_line = summary_lines.next().unwrap_or("");

    let summary = if first_line.len() > 60 {
        format!("{}...", &first_line[..57])
    } else {
        first_line.to_string()
    };

    let mut spans = vec![
        Span::styled(
            format!("    {icon} "),
            Style::default().fg(icon_color),
        ),
        Span::styled(
            name.to_string(),
            Style::default().fg(theme.accent).bold(),
        ),
        Span::styled(
            format!(" {summary}"),
            Style::default().fg(theme.text_tertiary),
        ),
    ];

    if *status == ToolStatus::Completed {
        if let Some(ms) = elapsed_ms {
            spans.push(Span::styled(
                format!("  ({})", format_elapsed(*ms)),
                Style::default().fg(theme.text_tertiary),
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
        let max_lines = if *status == ToolStatus::Running { 10 } else { 3 };
        let all_lines: Vec<&str> = out.lines().collect();
        let display_lines = if *status == ToolStatus::Running && all_lines.len() > max_lines {
            &all_lines[all_lines.len() - max_lines..]
        } else {
            &all_lines[..all_lines.len().min(max_lines)]
        };
        for line in display_lines {
            lines.push(Line::from(Span::styled(
                format!("      {line}"),
                Style::default().fg(theme.text_secondary),
            )));
        }
    }
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
