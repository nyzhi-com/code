use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::aesthetic::primitives;
use crate::aesthetic::tokens::*;
use crate::aesthetic::typography as ty;
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
    primitives::blur_overlay(frame, theme);

    let area = frame.area();
    let (done, _active, total) = state.progress();

    let popup_w = 70u16;
    let content_rows = total as u16 + SP_4;
    let popup_h = (content_rows + SP_4).min(area.height.saturating_sub(POPUP_MARGIN)).max(8);
    let popup_area = primitives::centered_popup(area, popup_w, popup_h);

    let title = format!("Todos ({done}/{total})");

    let enforcer_label = if state.enforcer_active {
        if state.enforce_count > 0 {
            format!("enforcer: active ({}/10)", state.enforce_count)
        } else {
            "enforcer: on".to_string()
        }
    } else {
        "enforcer: paused".to_string()
    };

    let enforcer_color = if state.enforcer_active {
        theme.success
    } else {
        theme.text_disabled
    };

    let footer_spans = vec![
        Span::styled(
            format!(" {enforcer_label} "),
            Style::default().fg(enforcer_color),
        ),
        Span::raw(" "),
        Span::styled("esc", Style::default().fg(theme.accent)),
        Span::styled(": close ", ty::disabled(theme)),
    ];

    let card = primitives::Card::new(theme)
        .title(&title)
        .title_bottom_spans(footer_spans);
    let inner = card.render_frame(frame, popup_area);

    if total == 0 {
        let empty = Paragraph::new(Line::from(vec![
            Span::styled("  No todos yet. ", ty::disabled(theme)),
            Span::styled(
                "The agent creates todos for multi-step tasks.",
                ty::muted(theme),
            ),
        ]))
        .style(ty::on_elevated(theme));
        frame.render_widget(empty, inner);
        return;
    }

    let bar_width = inner.width.saturating_sub(PAD_H * 2) as usize;
    let filled = if total > 0 {
        (done * bar_width) / total
    } else {
        0
    };
    let empty_bar = bar_width.saturating_sub(filled);

    let bar_line = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "\u{2588}".repeat(filled),
            Style::default().fg(theme.success),
        ),
        Span::styled(
            "\u{2591}".repeat(empty_bar),
            ty::disabled(theme),
        ),
        Span::styled(
            format!(" {done}/{total}"),
            ty::secondary(theme),
        ),
    ]);

    let sep = primitives::divider(inner.width.saturating_sub(PAD_H * 2), theme);
    let mut padded_sep = vec![Span::raw("  ")];
    padded_sep.extend(sep.spans);
    let sep_line = Line::from(padded_sep);

    let mut lines: Vec<Line> = vec![bar_line, sep_line];

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
            "completed" => ("\u{2713}", theme.success),
            "in_progress" => ("\u{25B8}", theme.warning),
            "cancelled" => ("\u{2717}", theme.text_disabled),
            _ if is_blocked => ("\u{2298}", theme.danger),
            _ => ("\u{25CB}", theme.text_secondary),
        };

        let content_style = match item.status.as_str() {
            "completed" => ty::disabled(theme),
            "in_progress" => ty::heading(theme),
            "cancelled" => ty::muted(theme),
            _ if is_blocked => ty::disabled(theme),
            _ => ty::secondary(theme),
        };

        let max_content = (inner.width as usize).saturating_sub(10);
        let truncated: String = if item.content.len() > max_content {
            format!("{}\u{2026}", &item.content[..max_content.saturating_sub(1)])
        } else {
            item.content.clone()
        };

        let mut spans = vec![
            Span::raw("  "),
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
                format!(" \u{2298}{}", pending_deps.join(",")),
                Style::default().fg(theme.danger),
            ));
        }

        lines.push(Line::from(spans));
    }

    let visible = inner.height as usize;
    let display_lines: Vec<Line> = lines.into_iter().take(visible).collect();

    let paragraph = Paragraph::new(display_lines).style(ty::on_elevated(theme));
    frame.render_widget(paragraph, inner);
}
