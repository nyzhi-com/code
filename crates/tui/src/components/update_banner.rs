use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::UpdateStatus;
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, status: &UpdateStatus, theme: &Theme) {
    match status {
        UpdateStatus::Available {
            new_version,
            current_version,
            ..
        } => {
            let text = Line::from(vec![
                Span::styled("  ↑ ", Style::default().fg(theme.accent).bold()),
                Span::styled(
                    format!("nyzhi v{new_version}"),
                    Style::default().fg(theme.text_primary).bold(),
                ),
                Span::styled(
                    format!(" available (current: v{current_version})  "),
                    Style::default().fg(theme.text_secondary),
                ),
                Span::styled("[u]", Style::default().fg(theme.accent).bold()),
                Span::styled(" Update  ", Style::default().fg(theme.text_secondary)),
                Span::styled("[s]", Style::default().fg(theme.text_secondary).bold()),
                Span::styled(" Skip  ", Style::default().fg(theme.text_secondary)),
                Span::styled("[i]", Style::default().fg(theme.text_secondary).bold()),
                Span::styled(" Ignore version", Style::default().fg(theme.text_secondary)),
            ]);
            let block = Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.border_default));
            let paragraph = Paragraph::new(text).block(block);
            frame.render_widget(paragraph, area);
        }
        UpdateStatus::Downloading { progress } => {
            let pct = progress.map(|p| format!(" {p}%")).unwrap_or_default();
            let text = Line::from(vec![
                Span::styled("  ⟳ ", Style::default().fg(theme.warning).bold()),
                Span::styled(
                    format!("Downloading update...{pct}"),
                    Style::default().fg(theme.text_primary),
                ),
            ]);
            let block = Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.border_default));
            frame.render_widget(Paragraph::new(text).block(block), area);
        }
        _ => {}
    }
}

pub fn height(status: &UpdateStatus) -> u16 {
    match status {
        UpdateStatus::Available { .. } | UpdateStatus::Downloading { .. } => 2,
        _ => 0,
    }
}
