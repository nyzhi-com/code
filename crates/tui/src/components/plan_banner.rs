use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::aesthetic::borders;
use crate::aesthetic::typography as ty;
use crate::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, theme: &Theme) {
    let text = Line::from(vec![
        Span::styled(
            format!(" {} ", borders::BAR_CHAR),
            Style::default().fg(theme.warning).bold(),
        ),
        Span::styled(
            "Plan Mode",
            ty::warning_style(theme).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " \u{2014} read-only analysis, no edits or commands",
            ty::secondary(theme),
        ),
    ]);
    let paragraph = Paragraph::new(text).style(ty::on_surface(theme));
    frame.render_widget(paragraph, area);
}

pub fn height(plan_mode: bool) -> u16 {
    if plan_mode {
        1
    } else {
        0
    }
}
