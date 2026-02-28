use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::theme::Theme;
use super::borders;
use super::tokens::*;

// ---------------------------------------------------------------------------
// BlurOverlay -- dims background behind modals
// ---------------------------------------------------------------------------

pub fn blur_overlay(frame: &mut ratatui::Frame, theme: &Theme) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(
            Style::default()
                .bg(theme.bg_sunken)
                .add_modifier(Modifier::DIM),
        ),
        area,
    );
}

// ---------------------------------------------------------------------------
// Card -- bordered container with optional title and accent bar
// ---------------------------------------------------------------------------

pub struct Card<'a> {
    pub title: Option<&'a str>,
    pub title_bottom: Option<Vec<Span<'a>>>,
    pub accent_color: Option<Color>,
    pub bg: Color,
    pub border_color: Color,
}

impl<'a> Card<'a> {
    pub fn new(theme: &Theme) -> Self {
        Self {
            title: None,
            title_bottom: None,
            accent_color: None,
            bg: theme.bg_elevated,
            border_color: theme.border_strong,
        }
    }

    pub fn title(mut self, t: &'a str) -> Self {
        self.title = Some(t);
        self
    }

    pub fn title_bottom_spans(mut self, spans: Vec<Span<'a>>) -> Self {
        self.title_bottom = Some(spans);
        self
    }

    pub fn accent(mut self, color: Color) -> Self {
        self.accent_color = Some(color);
        self
    }

    pub fn bg_color(mut self, c: Color) -> Self {
        self.bg = c;
        self
    }

    pub fn border(mut self, c: Color) -> Self {
        self.border_color = c;
        self
    }

    pub fn render_frame(&self, frame: &mut ratatui::Frame, area: Rect) -> Rect {
        frame.render_widget(Clear, area);

        let mut block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg));

        if let Some(t) = self.title {
            block = block
                .title(Line::from(Span::styled(
                    format!(" {t} "),
                    Style::default().fg(self.border_color).bold(),
                )))
                .title_alignment(Alignment::Center);
        }

        if let Some(ref spans) = self.title_bottom {
            block = block.title_bottom(
                Line::from(spans.clone()).alignment(Alignment::Right),
            );
        }

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if let Some(color) = self.accent_color {
            borders::accent_bar(frame, Rect::new(inner.x, inner.y, 1, inner.height), color, self.bg);
            Rect::new(
                inner.x + ACCENT_GUTTER,
                inner.y,
                inner.width.saturating_sub(ACCENT_GUTTER),
                inner.height,
            )
        } else {
            inner
        }
    }
}

// ---------------------------------------------------------------------------
// Pill -- inline styled label  [ text ]
// ---------------------------------------------------------------------------

pub fn pill<'a>(text: &str, fg: Color, bg: Color) -> Span<'a> {
    Span::styled(
        format!(" {text} "),
        Style::default().fg(fg).bg(bg).bold(),
    )
}

pub fn pill_outline<'a>(text: &str, color: Color) -> Span<'a> {
    Span::styled(
        format!(" {text} "),
        Style::default().fg(color).bold(),
    )
}

// ---------------------------------------------------------------------------
// Badge -- subtle indicator with no background
// ---------------------------------------------------------------------------

pub fn badge<'a>(text: &str, color: Color) -> Span<'a> {
    Span::styled(text.to_string(), Style::default().fg(color))
}

// ---------------------------------------------------------------------------
// Divider -- horizontal line with optional label
// ---------------------------------------------------------------------------

pub fn divider<'a>(width: u16, theme: &Theme) -> Line<'a> {
    Line::from(Span::styled(
        "─".repeat(width as usize),
        Style::default().fg(theme.border_default),
    ))
}

pub fn divider_label<'a>(label: &str, width: u16, theme: &Theme) -> Line<'a> {
    borders::h_separator_label(label, width, theme)
}

// ---------------------------------------------------------------------------
// Panel -- full-height side panel with left-edge border and title
// ---------------------------------------------------------------------------

pub struct Panel<'a> {
    pub title: Option<&'a str>,
    pub title_bottom: Option<Vec<Span<'a>>>,
    pub bg: Color,
    pub border_color: Color,
}

impl<'a> Panel<'a> {
    pub fn new(theme: &Theme) -> Self {
        Self {
            title: None,
            title_bottom: None,
            bg: theme.bg_page,
            border_color: theme.border_default,
        }
    }

    pub fn title(mut self, t: &'a str) -> Self {
        self.title = Some(t);
        self
    }

    pub fn title_bottom_spans(mut self, spans: Vec<Span<'a>>) -> Self {
        self.title_bottom = Some(spans);
        self
    }

    pub fn bg_color(mut self, c: Color) -> Self {
        self.bg = c;
        self
    }

    pub fn border_color(mut self, c: Color) -> Self {
        self.border_color = c;
        self
    }

    pub fn render_frame(&self, frame: &mut ratatui::Frame, area: Rect) -> Rect {
        let mut block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg));

        if let Some(t) = self.title {
            block = block
                .title(
                    Line::from(Span::styled(
                        format!(" {t} "),
                        Style::default().fg(self.border_color).bold(),
                    ))
                    .alignment(Alignment::Center),
                );
        }

        if let Some(ref spans) = self.title_bottom {
            block = block.title_bottom(
                Line::from(spans.clone()).alignment(Alignment::Right),
            );
        }

        let inner = block.inner(area);
        frame.render_widget(block, area);
        inner
    }
}

// ---------------------------------------------------------------------------
// Centered popup area calculator
// ---------------------------------------------------------------------------

pub fn centered_popup(area: Rect, w: u16, h: u16) -> Rect {
    let cw = w.min(area.width.saturating_sub(POPUP_MARGIN * 2));
    let ch = h.min(area.height.saturating_sub(POPUP_MARGIN));
    let x = area.x + (area.width.saturating_sub(cw)) / 2;
    let y = area.y + (area.height.saturating_sub(ch)) / 2;
    Rect::new(x, y, cw, ch)
}
