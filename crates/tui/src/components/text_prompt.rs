use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPromptKind {
    ExaApiKey,
    UserQuestionCustom,
}

#[derive(Debug, Clone)]
pub struct TextPromptState {
    pub kind: TextPromptKind,
    pub title: String,
    pub description: Vec<String>,
    pub placeholder: String,
    pub value: String,
    pub cursor_pos: usize,
    pub masked: bool,
    pub revealed: bool,
}

pub enum TextPromptAction {
    None,
    Submit(String),
    Cancel,
}

impl TextPromptState {
    pub fn new(
        kind: TextPromptKind,
        title: &str,
        description: &[&str],
        placeholder: &str,
        masked: bool,
    ) -> Self {
        Self {
            kind,
            title: title.to_string(),
            description: description.iter().map(|s| s.to_string()).collect(),
            placeholder: placeholder.to_string(),
            value: String::new(),
            cursor_pos: 0,
            masked,
            revealed: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> TextPromptAction {
        match key.code {
            KeyCode::Esc => TextPromptAction::Cancel,
            KeyCode::Enter => {
                if !self.value.is_empty() {
                    TextPromptAction::Submit(self.value.clone())
                } else {
                    TextPromptAction::None
                }
            }
            KeyCode::Tab => {
                if self.masked {
                    self.revealed = !self.revealed;
                }
                TextPromptAction::None
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.value.remove(self.cursor_pos);
                }
                TextPromptAction::None
            }
            KeyCode::Delete => {
                if self.cursor_pos < self.value.len() {
                    self.value.remove(self.cursor_pos);
                }
                TextPromptAction::None
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
                TextPromptAction::None
            }
            KeyCode::Right => {
                if self.cursor_pos < self.value.len() {
                    self.cursor_pos += 1;
                }
                TextPromptAction::None
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                TextPromptAction::None
            }
            KeyCode::End => {
                self.cursor_pos = self.value.len();
                TextPromptAction::None
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'v' {
                    // Paste support would need clipboard access
                    TextPromptAction::None
                } else {
                    self.value.insert(self.cursor_pos, c);
                    self.cursor_pos += 1;
                    TextPromptAction::None
                }
            }
            _ => TextPromptAction::None,
        }
    }
}

pub fn draw(frame: &mut Frame, state: &TextPromptState, theme: &Theme) {
    let area = frame.area();

    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let desc_lines = state.description.len() as u16;
    let popup_height = (8 + desc_lines).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let mut hint_spans: Vec<Span> = vec![
        Span::styled(" enter", Style::default().fg(theme.accent)),
        Span::styled(" confirm  ", Style::default().fg(theme.text_disabled)),
        Span::styled("esc", Style::default().fg(theme.accent)),
        Span::styled(" cancel", Style::default().fg(theme.text_disabled)),
    ];
    if state.masked {
        hint_spans.push(Span::styled("  tab", Style::default().fg(theme.accent)));
        hint_spans.push(Span::styled(" reveal ", Style::default().fg(theme.text_disabled)));
    } else {
        hint_spans.push(Span::styled(" ", Style::default()));
    }

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_strong))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(&state.title, Style::default().fg(theme.accent).bold()),
            Span::raw(" "),
        ]))
        .title_alignment(Alignment::Center)
        .title_bottom(Line::from(hint_spans).alignment(Alignment::Right))
        .style(Style::default().bg(theme.bg_elevated));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let content = Rect::new(inner.x + 1, inner.y, inner.width.saturating_sub(2), inner.height);

    let mut y_offset = 0u16;

    for desc in &state.description {
        if y_offset < content.height {
            let desc_area = Rect::new(content.x, content.y + y_offset, content.width, 1);
            frame.render_widget(
                Paragraph::new(desc.as_str())
                    .style(Style::default().fg(theme.text_secondary).bg(theme.bg_elevated)),
                desc_area,
            );
            y_offset += 1;
        }
    }

    if !state.description.is_empty() {
        y_offset += 1;
    }

    if y_offset < content.height {
        let label_area = Rect::new(content.x, content.y + y_offset, content.width, 1);
        let label_text = match state.kind {
            TextPromptKind::UserQuestionCustom => "Your answer:",
            _ => "API Key:",
        };
        frame.render_widget(
            Paragraph::new(label_text)
                .style(Style::default().fg(theme.text_primary).bold().bg(theme.bg_elevated)),
            label_area,
        );
        y_offset += 1;
    }

    if y_offset < content.height {
        let input_area = Rect::new(content.x, content.y + y_offset, content.width, 1);

        let display_value = if state.value.is_empty() {
            state.placeholder.clone()
        } else if state.masked && !state.revealed {
            "\u{25CF}".repeat(state.value.len())
        } else {
            state.value.clone()
        };

        let style = if state.value.is_empty() {
            Style::default().fg(theme.text_disabled).bg(theme.bg_page)
        } else {
            Style::default().fg(theme.text_primary).bg(theme.bg_page)
        };

        let visible_width = content.width as usize;
        let display = if display_value.len() > visible_width {
            let start = display_value.len().saturating_sub(visible_width);
            display_value[start..].to_string()
        } else {
            display_value
        };

        frame.render_widget(
            Paragraph::new(display).style(style),
            input_area,
        );

        frame.set_cursor_position(Position::new(
            input_area.x + state.cursor_pos.min(visible_width) as u16,
            input_area.y,
        ));
    }
}
