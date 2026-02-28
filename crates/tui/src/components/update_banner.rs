use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::aesthetic::borders;
use crate::aesthetic::typography as ty;
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
                Span::styled(
                    format!("  {} ", borders::BAR_CHAR),
                    Style::default().fg(theme.accent).bold(),
                ),
                Span::styled(
                    format!("nyzhi v{new_version}"),
                    ty::heading(theme),
                ),
                Span::styled(
                    format!(" available (current: v{current_version})  "),
                    ty::secondary(theme),
                ),
                Span::styled("[u]", Style::default().fg(theme.accent).bold()),
                Span::styled(" Update  ", ty::secondary(theme)),
                Span::styled("[s]", ty::secondary(theme).add_modifier(Modifier::BOLD)),
                Span::styled(" Skip  ", ty::secondary(theme)),
                Span::styled("[i]", ty::secondary(theme).add_modifier(Modifier::BOLD)),
                Span::styled(" Ignore version", ty::secondary(theme)),
            ]);
            let block = Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.border_default));
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(ty::on_surface(theme));
            frame.render_widget(paragraph, area);
        }
        UpdateStatus::Downloading { progress } => {
            let pct = progress.map(|p| format!(" {p}%")).unwrap_or_default();
            let text = Line::from(vec![
                Span::styled(
                    format!("  {} ", borders::BAR_CHAR),
                    Style::default().fg(theme.warning).bold(),
                ),
                Span::styled(
                    format!("Downloading update...{pct}"),
                    ty::body(theme),
                ),
            ]);
            let block = Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.border_default));
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(ty::on_surface(theme));
            frame.render_widget(paragraph, area);
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
