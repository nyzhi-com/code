use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::aesthetic::primitives;
use crate::aesthetic::tokens::*;
use crate::aesthetic::typography as ty;
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
    primitives::blur_overlay(frame, theme);

    let area = frame.area();
    let desc_lines = state.description.len() as u16;
    let popup_h = (8 + desc_lines).min(area.height.saturating_sub(POPUP_MARGIN));
    let popup_w = 60u16;
    let popup_area = primitives::centered_popup(area, popup_w, popup_h);

    let mut hint_spans: Vec<Span> = vec![
        Span::styled(" enter", Style::default().fg(theme.accent)),
        Span::styled(" confirm  ", ty::disabled(theme)),
        Span::styled("esc", Style::default().fg(theme.accent)),
        Span::styled(" cancel", ty::disabled(theme)),
    ];
    if state.masked {
        hint_spans.push(Span::styled("  tab", Style::default().fg(theme.accent)));
        hint_spans.push(Span::styled(" reveal ", ty::disabled(theme)));
    } else {
        hint_spans.push(Span::raw(" "));
    }

    let card = primitives::Card::new(theme)
        .title(&state.title)
        .title_bottom_spans(hint_spans);
    let inner = card.render_frame(frame, popup_area);

    let content = Rect::new(
        inner.x + SP_1,
        inner.y,
        inner.width.saturating_sub(SP_1 * 2),
        inner.height,
    );

    let mut y_offset = 0u16;

    for desc in &state.description {
        if y_offset < content.height {
            let desc_area = Rect::new(content.x, content.y + y_offset, content.width, 1);
            frame.render_widget(
                Paragraph::new(desc.as_str())
                    .style(ty::secondary(theme).bg(theme.bg_elevated)),
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
                .style(ty::heading(theme).bg(theme.bg_elevated)),
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
            ty::disabled(theme).bg(theme.bg_page)
        } else {
            ty::body(theme).bg(theme.bg_page)
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
