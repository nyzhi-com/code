use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct SelectorItem {
    pub label: String,
    pub value: String,
    pub preview_color: Option<Color>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorKind {
    Theme,
    Accent,
    Model,
}

#[derive(Debug, Clone)]
pub struct SelectorState {
    pub kind: SelectorKind,
    pub title: String,
    pub items: Vec<SelectorItem>,
    pub cursor: usize,
    pub active_idx: Option<usize>,
}

pub enum SelectorAction {
    None,
    Select(String),
    Cancel,
}

impl SelectorState {
    pub fn new(kind: SelectorKind, title: &str, items: Vec<SelectorItem>, active_value: &str) -> Self {
        let active_idx = items.iter().position(|i| i.value == active_value);
        Self {
            kind,
            title: title.to_string(),
            items,
            cursor: active_idx.unwrap_or(0),
            active_idx,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SelectorAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                SelectorAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.cursor + 1 < self.items.len() {
                    self.cursor += 1;
                }
                SelectorAction::None
            }
            KeyCode::Enter => {
                if let Some(item) = self.items.get(self.cursor) {
                    SelectorAction::Select(item.value.clone())
                } else {
                    SelectorAction::Cancel
                }
            }
            KeyCode::Esc => SelectorAction::Cancel,
            _ => SelectorAction::None,
        }
    }
}

pub fn draw(frame: &mut Frame, selector: &SelectorState, theme: &Theme) {
    let area = frame.area();

    let item_count = selector.items.len() as u16;
    let popup_h = (item_count + 4).min(area.height.saturating_sub(4));
    let popup_w = 40u16.min(area.width.saturating_sub(8));

    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_strong))
        .title(Line::from(Span::styled(
            format!(" {} ", selector.title),
            Style::default().fg(theme.accent).bold(),
        )))
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg_elevated));

    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let visible_h = inner.height as usize;
    let scroll = if selector.cursor >= visible_h {
        selector.cursor - visible_h + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();
    for (i, item) in selector.items.iter().enumerate().skip(scroll).take(visible_h) {
        let is_cursor = i == selector.cursor;
        let is_active = selector.active_idx == Some(i);

        let marker = if is_active { "● " } else { "  " };
        let arrow = if is_cursor { "▸ " } else { "  " };

        let mut spans = vec![];

        if is_cursor {
            spans.push(Span::styled(arrow, Style::default().fg(theme.accent)));
        } else {
            spans.push(Span::styled(arrow, Style::default().fg(theme.text_disabled)));
        }

        let marker_color = if is_active { theme.accent } else { theme.text_disabled };
        spans.push(Span::styled(marker, Style::default().fg(marker_color)));

        if let Some(color) = item.preview_color {
            spans.push(Span::styled("█ ", Style::default().fg(color)));
        }

        let label_style = if is_cursor {
            Style::default().fg(theme.text_primary).bold()
        } else if is_active {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.text_secondary)
        };
        spans.push(Span::styled(item.label.clone(), label_style));

        lines.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(lines).style(Style::default().bg(theme.bg_elevated));
    frame.render_widget(paragraph, inner);
}
