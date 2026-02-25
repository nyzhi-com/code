use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::theme::Theme;
use nyzhi_core::planning::{PlanFile, TodoStatus};

#[derive(Debug, Default)]
pub struct PlanPanelState {
    pub plan: Option<PlanFile>,
    pub scroll: u16,
}

impl PlanPanelState {
    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn load(&mut self, plan: PlanFile) {
        self.plan = Some(plan);
        self.scroll = 0;
    }
}

pub fn draw(frame: &mut Frame, area: Rect, state: &PlanPanelState, theme: &Theme) {
    let (done, total) = state
        .plan
        .as_ref()
        .map(|p| p.progress())
        .unwrap_or((0, 0));

    let title = state
        .plan
        .as_ref()
        .map(|p| format!(" {} ", p.frontmatter.name))
        .unwrap_or_else(|| " Plan ".to_string());

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_default))
        .title(
            Line::from(Span::styled(
                title,
                Style::default().fg(theme.accent).bold(),
            ))
            .alignment(Alignment::Center),
        )
        .title_bottom(
            Line::from(vec![
                Span::styled(" ^P", Style::default().fg(theme.accent)),
                Span::styled(": close ", Style::default().fg(theme.text_disabled)),
            ])
            .alignment(Alignment::Right),
        )
        .style(Style::default().bg(theme.bg_page));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(plan) = &state.plan else {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No plan for this session.",
            Style::default().fg(theme.text_disabled),
        )))
        .style(Style::default().bg(theme.bg_page));
        frame.render_widget(empty, inner);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    if !plan.frontmatter.overview.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  {}", plan.frontmatter.overview),
            Style::default().fg(theme.text_secondary).italic(),
        )));
        lines.push(Line::from(""));
    }

    if total > 0 {
        let bar_width = inner.width.saturating_sub(4) as usize;
        let filled = (done * bar_width) / total.max(1);
        let empty_bar = bar_width.saturating_sub(filled);

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("█".repeat(filled), Style::default().fg(theme.success)),
            Span::styled(
                "░".repeat(empty_bar),
                Style::default().fg(theme.text_disabled),
            ),
            Span::styled(
                format!(" {done}/{total}"),
                Style::default().fg(theme.text_secondary),
            ),
        ]));
        lines.push(Line::from(""));

        for todo in &plan.frontmatter.todos {
            let (marker, marker_color) = match todo.status {
                TodoStatus::Completed => ("✓", theme.success),
                TodoStatus::InProgress => ("▸", theme.warning),
                TodoStatus::Cancelled => ("✗", theme.text_disabled),
                TodoStatus::Pending => ("○", theme.text_secondary),
            };

            let content_style = match todo.status {
                TodoStatus::Completed => Style::default().fg(theme.text_disabled),
                TodoStatus::InProgress => Style::default().fg(theme.text_primary).bold(),
                TodoStatus::Cancelled => Style::default().fg(theme.text_disabled).italic(),
                TodoStatus::Pending => Style::default().fg(theme.text_secondary),
            };

            let max_w = (inner.width as usize).saturating_sub(8);
            let display: String = if todo.content.len() > max_w {
                format!("{}…", &todo.content[..max_w.saturating_sub(1)])
            } else {
                todo.content.clone()
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{marker} "), Style::default().fg(marker_color)),
                Span::styled(display, content_style),
            ]));
        }
    }

    if !plan.body.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(inner.width.saturating_sub(4) as usize)),
            Style::default().fg(theme.border_default),
        )));
        lines.push(Line::from(""));

        let max_w = inner.width.saturating_sub(4) as usize;
        for body_line in plan.body.lines() {
            let styled = style_plan_line(body_line, theme);
            if body_line.len() > max_w && max_w > 0 {
                let mut remaining = body_line;
                while !remaining.is_empty() {
                    let take = remaining.len().min(max_w);
                    let chunk = &remaining[..take];
                    lines.push(Line::from(Span::styled(
                        format!("  {chunk}"),
                        Style::default().fg(theme.text_primary),
                    )));
                    remaining = &remaining[take..];
                }
            } else {
                lines.push(styled);
            }
        }
    }

    let visible = inner.height;
    let total_lines = lines.len() as u16;
    let scroll = state.scroll.min(total_lines.saturating_sub(visible));

    let paragraph = Paragraph::new(lines)
        .scroll((scroll, 0))
        .style(Style::default().bg(theme.bg_page));

    frame.render_widget(paragraph, inner);
}

fn style_plan_line<'a>(line: &str, theme: &Theme) -> Line<'a> {
    if line.starts_with("# ") || line.starts_with("## ") || line.starts_with("### ") {
        Line::from(Span::styled(
            format!("  {line}"),
            Style::default().fg(theme.accent).bold(),
        ))
    } else if line.starts_with("- ") || line.starts_with("* ") {
        Line::from(Span::styled(
            format!("  {line}"),
            Style::default().fg(theme.text_primary),
        ))
    } else {
        Line::from(Span::styled(
            format!("  {line}"),
            Style::default().fg(theme.text_secondary),
        ))
    }
}
