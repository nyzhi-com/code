use ratatui::prelude::*;
use ratatui::style::Color;

use crate::theme::Theme;

pub fn heading(theme: &Theme) -> Style {
    Style::default().fg(theme.text_primary).bold()
}

pub fn subheading(theme: &Theme) -> Style {
    Style::default().fg(theme.accent).bold()
}

pub fn body(theme: &Theme) -> Style {
    Style::default().fg(theme.text_primary)
}

pub fn body_bold(theme: &Theme) -> Style {
    Style::default().fg(theme.text_primary).bold()
}

pub fn secondary(theme: &Theme) -> Style {
    Style::default().fg(theme.text_secondary)
}

pub fn caption(theme: &Theme) -> Style {
    Style::default().fg(theme.text_tertiary)
}

pub fn mono(theme: &Theme) -> Style {
    Style::default().fg(theme.text_secondary)
}

pub fn danger(theme: &Theme) -> Style {
    Style::default().fg(theme.danger).bold()
}

pub fn warning_style(theme: &Theme) -> Style {
    Style::default().fg(theme.warning).bold()
}

pub fn success(theme: &Theme) -> Style {
    Style::default().fg(theme.success).bold()
}

pub fn muted(theme: &Theme) -> Style {
    Style::default()
        .fg(theme.text_disabled)
        .add_modifier(Modifier::ITALIC)
}

pub fn disabled(theme: &Theme) -> Style {
    Style::default().fg(theme.text_disabled)
}

pub fn accent(color: Color) -> Style {
    Style::default().fg(color).bold()
}

pub fn on_surface(theme: &Theme) -> Style {
    Style::default().bg(theme.bg_surface)
}

pub fn on_elevated(theme: &Theme) -> Style {
    Style::default().bg(theme.bg_elevated)
}

pub fn on_page(theme: &Theme) -> Style {
    Style::default().bg(theme.bg_page)
}
