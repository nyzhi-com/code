use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::aesthetic::primitives;
use crate::aesthetic::tokens::*;
use crate::aesthetic::typography as ty;
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
        .map(|p| p.frontmatter.name.clone())
        .unwrap_or_else(|| "Plan".to_string());

    let footer_spans = vec![
        Span::styled(" ^P", Style::default().fg(theme.accent).bold()),
        Span::styled(": close ", ty::disabled(theme)),
    ];

    let panel = primitives::Panel::new(theme)
        .title(&title)
        .border_color(theme.accent)
        .title_bottom_spans(footer_spans);
    let inner = panel.render_frame(frame, area);

    let Some(plan) = &state.plan else {
        let empty = Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled("No plan for this session.", ty::disabled(theme)),
        ]))
        .style(ty::on_page(theme));
        frame.render_widget(empty, inner);
        return;
    };

    let inner_w = inner.width as usize;
    let mut lines: Vec<Line> = Vec::new();

    if !plan.frontmatter.overview.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                plan.frontmatter.overview.clone(),
                ty::muted(theme).add_modifier(Modifier::ITALIC),
            ),
        ]));
        lines.push(Line::from(""));
    }

    if total > 0 {
        let bar_width = inner_w.saturating_sub(PAD_H as usize * 2 + 12);
        let filled = (done * bar_width) / total.max(1);
        let empty_bar = bar_width.saturating_sub(filled);
        let pct = (done * 100) / total.max(1);

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "\u{2588}".repeat(filled),
                Style::default().fg(theme.accent),
            ),
            Span::styled(
                "\u{2591}".repeat(empty_bar),
                Style::default().fg(theme.border_default),
            ),
            Span::styled(
                format!(" {pct}% ({done}/{total})"),
                ty::secondary(theme),
            ),
        ]));
        lines.push(Line::from(""));

        for todo in &plan.frontmatter.todos {
            let is_active = matches!(todo.status, TodoStatus::InProgress);

            let row_bg = if is_active {
                theme.accent
            } else {
                theme.bg_page
            };
            let primary_fg = if is_active {
                theme.bg_page
            } else {
                theme.text_primary
            };

            let (marker, marker_fg) = match todo.status {
                TodoStatus::Completed => (
                    "\u{2713}",
                    if is_active { theme.bg_page } else { theme.success },
                ),
                TodoStatus::InProgress => ("\u{25B8}", theme.bg_page),
                TodoStatus::Cancelled => ("\u{2717}", theme.text_disabled),
                TodoStatus::Pending => ("\u{25CB}", theme.text_secondary),
            };

            let content_style = match todo.status {
                TodoStatus::Completed => Style::default()
                    .fg(theme.text_disabled)
                    .bg(row_bg)
                    .add_modifier(Modifier::CROSSED_OUT),
                TodoStatus::InProgress => Style::default().fg(primary_fg).bg(row_bg).bold(),
                TodoStatus::Cancelled => Style::default()
                    .fg(theme.text_disabled)
                    .bg(row_bg)
                    .add_modifier(Modifier::CROSSED_OUT),
                TodoStatus::Pending => Style::default().fg(primary_fg).bg(row_bg),
            };

            let max_w = inner_w.saturating_sub(8);
            let display: String = if todo.content.len() > max_w {
                format!("{}\u{2026}", &todo.content[..max_w.saturating_sub(1)])
            } else {
                todo.content.clone()
            };

            let mut spans = vec![
                Span::styled(" ", Style::default().bg(row_bg)),
                Span::styled(
                    format!(" {marker} "),
                    Style::default().fg(marker_fg).bg(row_bg).bold(),
                ),
                Span::styled(display.clone(), content_style),
            ];

            let used: usize = spans.iter().map(|s| s.width()).sum();
            let trail = inner_w.saturating_sub(used);
            if trail > 0 {
                spans.push(Span::styled(
                    " ".repeat(trail),
                    Style::default().bg(row_bg),
                ));
            }

            lines.push(Line::from(spans));
        }
    }

    if !plan.body.is_empty() {
        lines.push(Line::from(""));
        let sep = primitives::divider(inner.width.saturating_sub(PAD_H * 2), theme);
        let mut padded_sep = vec![Span::raw("  ")];
        padded_sep.extend(sep.spans);
        lines.push(Line::from(padded_sep));
        lines.push(Line::from(""));

        let max_w = inner_w.saturating_sub(PAD_H as usize * 2);
        for body_line in plan.body.lines() {
            let styled = style_plan_line(body_line, theme, inner_w);
            if body_line.len() > max_w && max_w > 0 {
                let mut remaining = body_line;
                while !remaining.is_empty() {
                    let take = remaining.len().min(max_w);
                    let chunk = &remaining[..take];
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(chunk.to_string(), ty::body(theme)),
                    ]));
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
        .style(ty::on_page(theme));

    frame.render_widget(paragraph, inner);
}

fn style_plan_line<'a>(line: &str, theme: &Theme, _inner_w: usize) -> Line<'a> {
    if line.starts_with("# ") || line.starts_with("## ") || line.starts_with("### ") {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                line.to_string(),
                Style::default().fg(theme.text_tertiary).bold(),
            ),
        ])
    } else if line.starts_with("- ") || line.starts_with("* ") {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(line.to_string(), ty::body(theme)),
        ])
    } else {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(line.to_string(), ty::secondary(theme)),
        ])
    }
}
