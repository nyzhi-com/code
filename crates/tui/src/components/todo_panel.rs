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

    let popup_w = (POPUP_MAX_W_PCT as u32 * area.width as u32 / 100) as u16;
    let popup_w = popup_w.min(area.width.saturating_sub(POPUP_MARGIN));
    let content_rows = total as u16 + SP_4;
    let popup_h = (content_rows + SP_4 + SP_4)
        .min(area.height.saturating_sub(POPUP_MARGIN))
        .max(10);
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
        Span::styled("esc", Style::default().fg(theme.accent).bold()),
        Span::styled(": close ", ty::disabled(theme)),
    ];

    let card = primitives::Card::new(theme)
        .title(&title)
        .border(theme.accent)
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

    let inner_w = inner.width as usize;
    let bar_width = inner_w.saturating_sub(PAD_H as usize * 2 + SP_8 as usize);
    let filled = if total > 0 {
        (done * bar_width) / total
    } else {
        0
    };
    let empty_bar = bar_width.saturating_sub(filled);
    let pct = if total > 0 {
        (done * 100) / total
    } else {
        0
    };

    let bar_line = Line::from(vec![
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
    ]);

    let sep = primitives::divider(inner.width, theme);

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

        let is_active = item.status == "in_progress";

        let row_bg = if is_active {
            theme.accent
        } else if is_blocked {
            // Subtle danger tint
            if let (Color::Rgb(r, g, b), Color::Rgb(dr, _dg, _db)) =
                (theme.bg_elevated, theme.danger)
            {
                Color::Rgb(
                    r.saturating_add(dr / 8),
                    g,
                    b,
                )
            } else {
                theme.bg_elevated
            }
        } else {
            theme.bg_elevated
        };

        let primary_fg = if is_active {
            theme.bg_page
        } else {
            theme.text_primary
        };

        let (marker, marker_fg) = match item.status.as_str() {
            "completed" => ("\u{2713}", if is_active { theme.bg_page } else { theme.success }),
            "in_progress" => ("\u{25B8}", theme.bg_page),
            "cancelled" => ("\u{2717}", theme.text_disabled),
            _ if is_blocked => ("\u{2298}", theme.danger),
            _ => ("\u{25CB}", theme.text_secondary),
        };

        let content_style = match item.status.as_str() {
            "completed" => Style::default()
                .fg(theme.text_disabled)
                .bg(row_bg)
                .add_modifier(Modifier::CROSSED_OUT),
            "in_progress" => Style::default().fg(primary_fg).bg(row_bg).bold(),
            "cancelled" => Style::default()
                .fg(theme.text_disabled)
                .bg(row_bg)
                .add_modifier(Modifier::CROSSED_OUT),
            _ if is_blocked => Style::default().fg(theme.text_disabled).bg(row_bg),
            _ => Style::default().fg(primary_fg).bg(row_bg),
        };

        let max_content = inner_w.saturating_sub(SP_8 as usize);
        let truncated: String = if item.content.len() > max_content {
            format!("{}\u{2026}", &item.content[..max_content.saturating_sub(1)])
        } else {
            item.content.clone()
        };

        let mut spans = vec![
            Span::styled(" ", Style::default().bg(row_bg)),
            Span::styled(
                format!(" {marker} "),
                Style::default().fg(marker_fg).bg(row_bg).bold(),
            ),
            Span::styled(truncated.clone(), content_style),
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
                Style::default().fg(theme.danger).bg(row_bg),
            ));
        }

        // Fill trailing space for full-width row background
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

    let visible = inner.height as usize;
    let display_lines: Vec<Line> = lines.into_iter().take(visible).collect();

    let paragraph = Paragraph::new(display_lines).style(ty::on_elevated(theme));
    frame.render_widget(paragraph, inner);
}
