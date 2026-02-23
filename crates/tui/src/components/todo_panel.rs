use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct TodoPanelItem {
    pub id: String,
    pub content: String,
    pub status: String,
    pub blocked_by: Vec<String>,
}

#[derive(Debug)]
pub struct TodoPanelState {
    pub items: Vec<TodoPanelItem>,
    pub scroll: u16,
    pub enforcer_active: bool,
    pub enforce_count: u32,
}

impl TodoPanelState {
    pub fn progress(&self) -> (usize, usize, usize) {
        let total = self.items.len();
        let done = self
            .items
            .iter()
            .filter(|t| t.status == "completed" || t.status == "cancelled")
            .count();
        let active = self
            .items
            .iter()
            .filter(|t| t.status == "in_progress")
            .count();
        (done, active, total)
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        let max = (self.items.len() as u16).saturating_sub(1);
        if self.scroll < max {
            self.scroll += 1;
        }
    }
}

pub fn draw(frame: &mut Frame, state: &TodoPanelState, theme: &Theme) {
    let area = frame.area();
    let (done, _active, total) = state.progress();

    let popup_w = 70u16.min(area.width.saturating_sub(8));
    let content_rows = total as u16 + 4;
    let popup_h = (content_rows + 4).min(area.height.saturating_sub(4)).max(8);

    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    let title = format!(" Todos ({done}/{total}) ");

    let enforcer_label = if state.enforcer_active {
        if state.enforce_count > 0 {
            format!("enforcer: active ({}/10)", state.enforce_count)
        } else {
            "enforcer: on".to_string()
        }
    } else {
        "enforcer: paused".to_string()
    };

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_strong))
        .title(
            Line::from(Span::styled(
                title,
                Style::default().fg(theme.accent).bold(),
            ))
            .alignment(Alignment::Center),
        )
        .title_bottom(
            Line::from(vec![
                Span::styled(
                    format!(" {enforcer_label} "),
                    Style::default().fg(if state.enforcer_active {
                        theme.success
                    } else {
                        theme.text_disabled
                    }),
                ),
                Span::raw(" "),
                Span::styled("esc", Style::default().fg(theme.accent)),
                Span::styled(": close ", Style::default().fg(theme.text_disabled)),
            ])
            .alignment(Alignment::Right),
        )
        .style(Style::default().bg(theme.bg_elevated));

    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    if total == 0 {
        let empty = Paragraph::new(Line::from(vec![
            Span::styled("  No todos yet. ", Style::default().fg(theme.text_disabled)),
            Span::styled(
                "The agent creates todos for multi-step tasks.",
                Style::default().fg(theme.text_disabled).italic(),
            ),
        ]))
        .style(Style::default().bg(theme.bg_elevated));
        frame.render_widget(empty, inner);
        return;
    }

    let bar_width = inner.width.saturating_sub(4) as usize;
    let filled = if total > 0 {
        (done * bar_width) / total
    } else {
        0
    };
    let empty_bar = bar_width.saturating_sub(filled);

    let bar_line = Line::from(vec![
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
    ]);

    let sep = Line::from(Span::styled(
        format!("  {}", "─".repeat(inner.width.saturating_sub(4) as usize)),
        Style::default().fg(theme.border_default),
    ));

    let mut lines: Vec<Line> = vec![bar_line, sep];

    let completed_ids: std::collections::HashSet<&str> = state
        .items
        .iter()
        .filter(|t| t.status == "completed")
        .map(|t| t.id.as_str())
        .collect();

    for item in state.items.iter().skip(state.scroll as usize) {
        let is_blocked = !item.blocked_by.is_empty()
            && !item
                .blocked_by
                .iter()
                .all(|dep| completed_ids.contains(dep.as_str()));

        let (marker, marker_color) = match item.status.as_str() {
            "completed" => ("✓", theme.success),
            "in_progress" => ("▸", theme.warning),
            "cancelled" => ("✗", theme.text_disabled),
            _ if is_blocked => ("⊘", theme.danger),
            _ => ("○", theme.text_secondary),
        };

        let content_style = match item.status.as_str() {
            "completed" => Style::default().fg(theme.text_disabled),
            "in_progress" => Style::default().fg(theme.text_primary).bold(),
            "cancelled" => Style::default().fg(theme.text_disabled).italic(),
            _ if is_blocked => Style::default().fg(theme.text_disabled),
            _ => Style::default().fg(theme.text_secondary),
        };

        let max_content = (inner.width as usize).saturating_sub(10);
        let truncated: String = if item.content.len() > max_content {
            format!("{}…", &item.content[..max_content.saturating_sub(1)])
        } else {
            item.content.clone()
        };

        let mut spans = vec![
            Span::styled("  ", Style::default()),
            Span::styled(format!("{marker} "), Style::default().fg(marker_color)),
            Span::styled(truncated, content_style),
        ];

        if is_blocked {
            let pending_deps: Vec<&str> = item
                .blocked_by
                .iter()
                .filter(|dep| !completed_ids.contains(dep.as_str()))
                .map(|s| s.as_str())
                .collect();
            spans.push(Span::styled(
                format!(" ⊘{}", pending_deps.join(",")),
                Style::default().fg(theme.danger),
            ));
        }

        lines.push(Line::from(spans));
    }

    let visible = inner.height as usize;
    let display_lines: Vec<Line> = lines.into_iter().take(visible).collect();

    let paragraph = Paragraph::new(display_lines).style(Style::default().bg(theme.bg_elevated));
    frame.render_widget(paragraph, inner);
}
