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
    pub fn new(kind: TextPromptKind, title: &str, description: &[&str], placeholder: &str, masked: bool) -> Self {
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

    let popup_width = 58u16.min(area.width.saturating_sub(4));
    let popup_height = (7 + state.description.len() as u16).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(&state.title, Style::default().fg(theme.accent).bold()),
            Span::raw(" "),
        ]))
        .title_alignment(Alignment::Center);

    frame.render_widget(block, popup_area);

    let inner = Rect::new(
        popup_area.x + 2,
        popup_area.y + 1,
        popup_area.width.saturating_sub(4),
        popup_area.height.saturating_sub(2),
    );

    let mut y_offset = 0u16;

    // Description lines
    for desc in &state.description {
        if y_offset < inner.height {
            let desc_area = Rect::new(inner.x, inner.y + y_offset, inner.width, 1);
            let text = Paragraph::new(desc.as_str())
                .style(Style::default().fg(theme.text_secondary));
            frame.render_widget(text, desc_area);
            y_offset += 1;
        }
    }

    if !state.description.is_empty() {
        y_offset += 1;
    }

    // Label
    if y_offset < inner.height {
        let label_area = Rect::new(inner.x, inner.y + y_offset, inner.width, 1);
        let label_text = match state.kind {
            TextPromptKind::UserQuestionCustom => "Your answer:",
            _ => "API Key:",
        };
        let label = Paragraph::new(label_text)
            .style(Style::default().fg(theme.text_primary));
        frame.render_widget(label, label_area);
        y_offset += 1;
    }

    // Input field
    if y_offset < inner.height {
        let input_area = Rect::new(inner.x, inner.y + y_offset, inner.width, 1);

        let display_value = if state.value.is_empty() {
            state.placeholder.clone()
        } else if state.masked && !state.revealed {
            let dots: String = "\u{25CF}".repeat(state.value.len());
            dots
        } else {
            state.value.clone()
        };

        let style = if state.value.is_empty() {
            Style::default().fg(theme.text_disabled)
        } else {
            Style::default().fg(theme.text_primary)
        };

        let input_block = Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.border_strong));

        let input_inner = input_block.inner(input_area);
        frame.render_widget(input_block, input_area);

        let visible_width = input_inner.width as usize;
        let display = if display_value.len() > visible_width {
            let start = display_value.len().saturating_sub(visible_width);
            display_value[start..].to_string()
        } else {
            display_value
        };

        let input_text = Paragraph::new(display).style(style);
        frame.render_widget(input_text, input_inner);

        y_offset += 2;
    }

    y_offset += 1;

    // Hint line
    if y_offset < inner.height {
        let hint_area = Rect::new(inner.x, inner.y + y_offset, inner.width, 1);
        let hints = vec![
            Span::styled("Enter", Style::default().fg(theme.accent)),
            Span::styled(": confirm  |  ", Style::default().fg(theme.text_disabled)),
            Span::styled("Esc", Style::default().fg(theme.accent)),
            Span::styled(": cancel  |  ", Style::default().fg(theme.text_disabled)),
            Span::styled("Tab", Style::default().fg(theme.accent)),
            Span::styled(": reveal", Style::default().fg(theme.text_disabled)),
        ];
        frame.render_widget(Paragraph::new(Line::from(hints)), hint_area);
    }
}
