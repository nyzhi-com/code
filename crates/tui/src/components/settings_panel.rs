use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::aesthetic::primitives;
use crate::aesthetic::tokens::*;
use crate::aesthetic::typography as ty;
use crate::theme::Theme;

#[derive(Debug, Clone)]
pub enum SettingKind {
    Toggle,
    Cycle { options: Vec<String> },
    SubMenu,
}

#[derive(Debug, Clone)]
pub struct SettingItem {
    pub key: String,
    pub label: String,
    pub description: String,
    pub kind: SettingKind,
    pub current_value: String,
}

#[derive(Debug, Clone)]
pub enum SettingsRow {
    Header(String),
    Item(SettingItem),
}

pub struct SettingsPanel {
    pub rows: Vec<SettingsRow>,
    pub cursor: usize,
}

pub enum SettingsAction {
    None,
    Toggle(String),
    CycleNext(String),
    CyclePrev(String),
    OpenSub(String),
    Close,
}

impl SettingsPanel {
    pub fn new(rows: Vec<SettingsRow>) -> Self {
        let cursor = rows
            .iter()
            .position(|r| matches!(r, SettingsRow::Item(_)))
            .unwrap_or(0);
        Self { rows, cursor }
    }

    fn move_cursor(&mut self, dir: i32) {
        let len = self.rows.len();
        let mut next = self.cursor as i32 + dir;
        loop {
            if next < 0 || next >= len as i32 {
                return;
            }
            if matches!(self.rows[next as usize], SettingsRow::Item(_)) {
                self.cursor = next as usize;
                return;
            }
            next += dir;
        }
    }

    fn focused_item(&self) -> Option<&SettingItem> {
        match self.rows.get(self.cursor) {
            Some(SettingsRow::Item(item)) => Some(item),
            _ => None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SettingsAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_cursor(-1);
                SettingsAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_cursor(1);
                SettingsAction::None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(item) = self.focused_item() {
                    let key = item.key.clone();
                    match &item.kind {
                        SettingKind::Toggle => SettingsAction::Toggle(key),
                        SettingKind::Cycle { .. } => SettingsAction::CycleNext(key),
                        SettingKind::SubMenu => SettingsAction::OpenSub(key),
                    }
                } else {
                    SettingsAction::None
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if let Some(item) = self.focused_item() {
                    let key = item.key.clone();
                    match &item.kind {
                        SettingKind::Cycle { .. } => SettingsAction::CycleNext(key),
                        SettingKind::SubMenu => SettingsAction::OpenSub(key),
                        _ => SettingsAction::None,
                    }
                } else {
                    SettingsAction::None
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(item) = self.focused_item() {
                    let key = item.key.clone();
                    match &item.kind {
                        SettingKind::Cycle { .. } => SettingsAction::CyclePrev(key),
                        _ => SettingsAction::None,
                    }
                } else {
                    SettingsAction::None
                }
            }
            KeyCode::Esc => SettingsAction::Close,
            _ => SettingsAction::None,
        }
    }

    pub fn update_value(&mut self, key: &str, new_value: &str) {
        for row in &mut self.rows {
            if let SettingsRow::Item(item) = row {
                if item.key == key {
                    item.current_value = new_value.to_string();
                    return;
                }
            }
        }
    }
}

pub fn draw(frame: &mut Frame, panel: &SettingsPanel, theme: &Theme) {
    primitives::blur_overlay(frame, theme);

    let area = frame.area();
    let row_count = panel.rows.len() as u16;
    let popup_h = (row_count + 6).min(area.height.saturating_sub(POPUP_MARGIN));
    let popup_w = 48u16;
    let popup_area = primitives::centered_popup(area, popup_w, popup_h);

    let desc = panel
        .focused_item()
        .map(|i| i.description.as_str())
        .unwrap_or("");

    let hint = match panel.focused_item().map(|i| &i.kind) {
        Some(SettingKind::Toggle) => "enter/space: toggle",
        Some(SettingKind::Cycle { .. }) => "\u{25C2} \u{25B8} cycle  enter: next",
        Some(SettingKind::SubMenu) => "enter: open",
        None => "",
    };

    let footer_spans = vec![
        Span::styled(" esc ", ty::disabled(theme)),
        Span::styled(hint, ty::disabled(theme)),
        Span::raw(" "),
    ];

    let card = primitives::Card::new(theme)
        .title("Settings")
        .border(theme.accent)
        .title_bottom_spans(footer_spans);
    let inner = card.render_frame(frame, popup_area);

    let sep_and_desc_h: u16 = if desc.is_empty() { 0 } else { SP_2 };
    let list_h = inner.height.saturating_sub(sep_and_desc_h);
    let list_area = Rect::new(inner.x, inner.y, inner.width, list_h);

    let cursor_pos = panel.cursor;
    let content_h = list_h as usize;
    let scroll = if cursor_pos >= content_h {
        cursor_pos - content_h + 1
    } else {
        0
    };

    let inner_w = list_area.width as usize;
    let label_col = 16usize.min(inner_w / 2);

    let mut lines: Vec<Line> = Vec::new();
    for (i, row) in panel.rows.iter().enumerate().skip(scroll).take(content_h) {
        match row {
            SettingsRow::Header(name) => {
                if i > 0 {
                    lines.push(Line::from(Span::styled(
                        " ".repeat(inner_w),
                        Style::default().bg(theme.bg_elevated),
                    )));
                }
                let label = format!(" {name}");
                let trail = inner_w.saturating_sub(label.len());
                lines.push(Line::from(vec![
                    Span::styled(
                        label,
                        Style::default()
                            .fg(theme.text_tertiary)
                            .bg(theme.bg_elevated)
                            .bold(),
                    ),
                    Span::styled(
                        " ".repeat(trail),
                        Style::default().bg(theme.bg_elevated),
                    ),
                ]));
            }
            SettingsRow::Item(item) => {
                let is_focused = i == panel.cursor;
                let row_bg = if is_focused { theme.accent } else { theme.bg_elevated };
                let primary_fg = if is_focused { theme.bg_page } else { theme.text_primary };
                let secondary_fg = if is_focused { theme.bg_elevated } else { theme.text_disabled };

                let arrow = if is_focused { "\u{25B8} " } else { "  " };
                let value_spans = render_value(item, theme, is_focused, row_bg);

                let label_len = item.label.len();
                let pad = if label_len < label_col {
                    label_col - label_len
                } else {
                    1
                };

                let mut spans = vec![
                    Span::styled(
                        arrow.to_string(),
                        Style::default().fg(primary_fg).bg(row_bg),
                    ),
                    Span::styled(
                        item.label.clone(),
                        Style::default().fg(primary_fg).bg(row_bg).bold(),
                    ),
                    Span::styled(
                        " ".repeat(pad),
                        Style::default().bg(row_bg),
                    ),
                ];
                spans.extend(value_spans);

                let used: usize = spans.iter().map(|s| s.width()).sum();
                let trail = inner_w.saturating_sub(used);
                if trail > 0 {
                    spans.push(Span::styled(
                        " ".repeat(trail),
                        Style::default().fg(secondary_fg).bg(row_bg),
                    ));
                }
                lines.push(Line::from(spans));
            }
        }
    }

    let paragraph = Paragraph::new(lines).style(ty::on_elevated(theme));
    frame.render_widget(paragraph, list_area);

    if !desc.is_empty() {
        let sep_area = Rect::new(
            inner.x,
            inner.y + list_h,
            inner.width,
            1,
        );
        frame.render_widget(
            Paragraph::new(primitives::divider(inner.width, theme))
                .style(ty::on_elevated(theme)),
            sep_area,
        );

        let desc_area = Rect::new(
            inner.x,
            inner.y + list_h + 1,
            inner.width,
            1,
        );
        let desc_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(desc, ty::muted(theme)),
        ]);
        let desc_p = Paragraph::new(desc_line).style(ty::on_elevated(theme));
        frame.render_widget(desc_p, desc_area);
    }
}

fn render_value<'a>(
    item: &SettingItem,
    theme: &Theme,
    is_focused: bool,
    row_bg: Color,
) -> Vec<Span<'a>> {
    let primary_fg = if is_focused { theme.bg_page } else { theme.text_tertiary };
    let secondary_fg = if is_focused { theme.bg_elevated } else { theme.text_disabled };

    match &item.kind {
        SettingKind::Toggle => {
            let (icon, fg) = if item.current_value == "On" {
                if is_focused {
                    ("[\u{2713}]", theme.bg_page)
                } else {
                    ("[\u{2713}]", theme.success)
                }
            } else if is_focused {
                ("[ ]", theme.bg_elevated)
            } else {
                ("[ ]", theme.text_disabled)
            };
            vec![Span::styled(
                icon.to_string(),
                Style::default().fg(fg).bg(row_bg).bold(),
            )]
        }
        SettingKind::Cycle { .. } => {
            vec![
                Span::styled(
                    "\u{25C2} ",
                    Style::default().fg(secondary_fg).bg(row_bg),
                ),
                Span::styled(
                    item.current_value.clone(),
                    Style::default().fg(primary_fg).bg(row_bg).bold(),
                ),
                Span::styled(
                    " \u{25B8}",
                    Style::default().fg(secondary_fg).bg(row_bg),
                ),
            ]
        }
        SettingKind::SubMenu => {
            vec![
                Span::styled(
                    item.current_value.clone(),
                    Style::default().fg(primary_fg).bg(row_bg),
                ),
                Span::styled(
                    " \u{25B8}",
                    Style::default().fg(secondary_fg).bg(row_bg),
                ),
            ]
        }
    }
}
