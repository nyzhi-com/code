use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct SelectorItem {
    pub label: String,
    pub value: String,
    pub preview_color: Option<Color>,
    pub is_header: bool,
    pub right_badge: Option<String>,
}

impl SelectorItem {
    pub fn entry(label: &str, value: &str) -> Self {
        Self {
            label: label.to_string(),
            value: value.to_string(),
            preview_color: None,
            is_header: false,
            right_badge: None,
        }
    }

    pub fn header(label: &str) -> Self {
        Self {
            label: label.to_string(),
            value: String::new(),
            preview_color: None,
            is_header: true,
            right_badge: None,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.preview_color = Some(color);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorKind {
    Theme,
    Accent,
    Model,
    Provider,
    ConnectMethod,
    ApiKeyInput,
    Command,
    Style,
    Trust,
    Session,
    CustomModelInput,
    UserQuestion,
    PlanTransition,
}

#[derive(Debug, Clone)]
pub struct SelectorState {
    pub kind: SelectorKind,
    pub title: String,
    pub items: Vec<SelectorItem>,
    pub cursor: usize,
    pub active_idx: Option<usize>,
    pub search: String,
    pub context_value: Option<String>,
}

pub enum SelectorAction {
    None,
    Select(String),
    Cancel,
    Tab,
    ViewAllProviders,
}

impl SelectorState {
    pub fn new(
        kind: SelectorKind,
        title: &str,
        items: Vec<SelectorItem>,
        active_value: &str,
    ) -> Self {
        let active_idx = items
            .iter()
            .position(|i| !i.is_header && i.value == active_value);
        let first_selectable = items.iter().position(|i| !i.is_header).unwrap_or(0);
        Self {
            kind,
            title: title.to_string(),
            items,
            cursor: active_idx.unwrap_or(first_selectable),
            active_idx,
            search: String::new(),
            context_value: None,
        }
    }

    fn filtered_indices(&self) -> Vec<usize> {
        if self.search.is_empty() {
            return (0..self.items.len()).collect();
        }
        let query = self.search.to_lowercase();
        let mut result = Vec::new();
        let mut last_header: Option<usize> = None;
        let mut header_needed = false;

        for (i, item) in self.items.iter().enumerate() {
            if item.is_header {
                if header_needed {
                    if let Some(h) = last_header {
                        result.push(h);
                    }
                }
                last_header = Some(i);
                header_needed = false;
            } else if item.label.to_lowercase().contains(&query)
                || item.value.to_lowercase().contains(&query)
            {
                if !header_needed {
                    if let Some(h) = last_header {
                        result.push(h);
                    }
                    header_needed = true;
                }
                result.push(i);
            }
        }
        result
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SelectorAction {
        let filtered = self.filtered_indices();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') if self.search.is_empty() => {
                loop {
                    if self.cursor == 0 {
                        break;
                    }
                    self.cursor -= 1;
                    if !self
                        .items
                        .get(self.cursor)
                        .map(|i| i.is_header)
                        .unwrap_or(false)
                    {
                        break;
                    }
                }
                SelectorAction::None
            }
            KeyCode::Down | KeyCode::Char('j') if self.search.is_empty() => {
                loop {
                    if self.cursor + 1 >= self.items.len() {
                        break;
                    }
                    self.cursor += 1;
                    if !self
                        .items
                        .get(self.cursor)
                        .map(|i| i.is_header)
                        .unwrap_or(false)
                    {
                        break;
                    }
                }
                SelectorAction::None
            }
            KeyCode::Up if !self.search.is_empty() => {
                if let Some(pos) = filtered.iter().position(|&i| i == self.cursor) {
                    for &idx in filtered[..pos].iter().rev() {
                        if !self.items[idx].is_header {
                            self.cursor = idx;
                            break;
                        }
                    }
                }
                SelectorAction::None
            }
            KeyCode::Down if !self.search.is_empty() => {
                if let Some(pos) = filtered.iter().position(|&i| i == self.cursor) {
                    for &idx in &filtered[pos + 1..] {
                        if !self.items[idx].is_header {
                            self.cursor = idx;
                            break;
                        }
                    }
                }
                SelectorAction::None
            }
            KeyCode::Enter => {
                if let Some(item) = self.items.get(self.cursor) {
                    if item.is_header {
                        SelectorAction::None
                    } else {
                        SelectorAction::Select(item.value.clone())
                    }
                } else {
                    SelectorAction::Cancel
                }
            }
            KeyCode::Tab => SelectorAction::Tab,
            KeyCode::Esc => SelectorAction::Cancel,
            KeyCode::Backspace => {
                self.search.pop();
                let filtered = self.filtered_indices();
                if let Some(&first) = filtered.iter().find(|&&i| !self.items[i].is_header) {
                    self.cursor = first;
                }
                SelectorAction::None
            }
            KeyCode::Char('a')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && self.kind == SelectorKind::Model =>
            {
                SelectorAction::ViewAllProviders
            }
            KeyCode::Char(c)
                if matches!(
                    self.kind,
                    SelectorKind::Provider
                        | SelectorKind::ApiKeyInput
                        | SelectorKind::Command
                        | SelectorKind::Session
                        | SelectorKind::Model
                        | SelectorKind::CustomModelInput
                        | SelectorKind::UserQuestion
                ) =>
            {
                self.search.push(c);
                let filtered = self.filtered_indices();
                if let Some(&first) = filtered.iter().find(|&&i| !self.items[i].is_header) {
                    self.cursor = first;
                }
                SelectorAction::None
            }
            _ => SelectorAction::None,
        }
    }
}

pub fn draw(frame: &mut Frame, selector: &SelectorState, theme: &Theme) {
    let area = frame.area();

    let overlay = Block::default().style(Style::default().bg(theme.bg_sunken).add_modifier(Modifier::DIM));
    frame.render_widget(overlay, area);

    let filtered = selector.filtered_indices();

    let item_count = filtered.len() as u16;
    let has_search = matches!(
        selector.kind,
        SelectorKind::Provider
            | SelectorKind::ApiKeyInput
            | SelectorKind::Command
            | SelectorKind::Session
            | SelectorKind::Model
            | SelectorKind::CustomModelInput
            | SelectorKind::UserQuestion
    );
    let search_rows = if has_search { 2 } else { 0 };
    let popup_h = (item_count + 4 + search_rows).min(area.height.saturating_sub(4));
    let base_w = match selector.kind {
        SelectorKind::Command | SelectorKind::Model | SelectorKind::UserQuestion => 60u16,
        _ => 50u16,
    };
    let popup_w = base_w.min(area.width.saturating_sub(8));

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

    let footer_spans: Vec<Span> = match selector.kind {
        SelectorKind::Model if selector.context_value.as_deref() != Some("thinking") => {
            vec![
                Span::styled(" ctrl+a", Style::default().fg(theme.accent)),
                Span::styled(": providers  ", Style::default().fg(theme.text_disabled)),
                Span::styled("tab", Style::default().fg(theme.accent)),
                Span::styled(": thinking  ", Style::default().fg(theme.text_disabled)),
                Span::styled("esc ", Style::default().fg(theme.text_disabled)),
            ]
        }
        SelectorKind::UserQuestion | SelectorKind::PlanTransition => {
            vec![
                Span::styled(" enter", Style::default().fg(theme.accent)),
                Span::styled(": select  ", Style::default().fg(theme.text_disabled)),
                Span::styled("esc ", Style::default().fg(theme.text_disabled)),
            ]
        }
        _ => {
            vec![Span::styled(
                " esc ",
                Style::default().fg(theme.text_disabled),
            )]
        }
    };
    let block = block.title_bottom(Line::from(footer_spans).alignment(Alignment::Right));

    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let mut content_area = inner;

    if has_search {
        let search_area = Rect::new(content_area.x, content_area.y, content_area.width, 1);
        let search_text = if selector.search.is_empty() {
            let placeholder = match selector.kind {
                SelectorKind::ApiKeyInput => "Enter API key...",
                SelectorKind::Command => "Type to filter commands...",
                SelectorKind::CustomModelInput => "e.g. anthropic/claude-sonnet-4",
                SelectorKind::Model => "Search...",
                SelectorKind::UserQuestion => "Type custom answer...",
                _ => "Search...",
            };
            Span::styled(placeholder, Style::default().fg(theme.text_disabled))
        } else if selector.kind == SelectorKind::ApiKeyInput {
            let masked: String = "*".repeat(selector.search.len());
            Span::styled(masked, Style::default().fg(theme.text_primary))
        } else {
            Span::styled(&selector.search, Style::default().fg(theme.text_primary))
        };
        let search_line = Line::from(vec![Span::styled("  ", Style::default()), search_text]);
        frame.render_widget(
            Paragraph::new(search_line).style(Style::default().bg(theme.bg_elevated)),
            search_area,
        );

        let sep_area = Rect::new(content_area.x, content_area.y + 1, content_area.width, 1);
        let sep = "─".repeat(content_area.width as usize);
        frame.render_widget(
            Paragraph::new(sep).style(
                Style::default()
                    .fg(theme.border_default)
                    .bg(theme.bg_elevated),
            ),
            sep_area,
        );
        content_area = Rect::new(
            content_area.x,
            content_area.y + 2,
            content_area.width,
            content_area.height.saturating_sub(2),
        );
    }

    let visible_h = content_area.height as usize;
    let cursor_in_filtered = filtered
        .iter()
        .position(|&i| i == selector.cursor)
        .unwrap_or(0);
    let scroll = if cursor_in_filtered >= visible_h {
        cursor_in_filtered - visible_h + 1
    } else {
        0
    };

    let inner_w = content_area.width as usize;

    let mut lines: Vec<Line> = Vec::new();
    for &orig_idx in filtered.iter().skip(scroll).take(visible_h) {
        let item = &selector.items[orig_idx];

        if item.is_header {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    item.label.clone(),
                    Style::default().fg(theme.text_primary).bold(),
                ),
            ]));
            continue;
        }

        let is_cursor = orig_idx == selector.cursor;
        let is_active = selector.active_idx == Some(orig_idx);

        let marker = if is_active { "● " } else { "  " };
        let arrow = if is_cursor { "▸ " } else { "  " };

        let mut spans = vec![];

        if is_cursor {
            spans.push(Span::styled(arrow, Style::default().fg(theme.accent)));
        } else {
            spans.push(Span::styled(
                arrow,
                Style::default().fg(theme.text_disabled),
            ));
        }

        let marker_color = if is_active {
            theme.accent
        } else {
            theme.text_disabled
        };
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

        let prefix_width = 4 + if item.preview_color.is_some() { 2 } else { 0 };

        if let Some(ref badge) = item.right_badge {
            let badge_style = Style::default().fg(theme.text_disabled);
            let label_len = item.label.chars().count();
            let badge_len = badge.chars().count();
            let avail = inner_w.saturating_sub(prefix_width);
            let gap = avail.saturating_sub(label_len + badge_len);
            spans.push(Span::styled(item.label.clone(), label_style));
            if gap > 0 {
                spans.push(Span::raw(" ".repeat(gap)));
            }
            spans.push(Span::styled(badge.clone(), badge_style));
        } else {
            spans.push(Span::styled(item.label.clone(), label_style));
        }

        lines.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(lines).style(Style::default().bg(theme.bg_elevated));
    frame.render_widget(paragraph, content_area);
}
