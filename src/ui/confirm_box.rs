use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::theme;

pub fn render(frame: &mut Frame, area: Rect, message: &str) {
    let popup_area = centered_rect(60, area);

    let block = Block::default()
        .title(Span::styled(" Confirm ", Style::default().fg(theme::TEXT)))
        .borders(Borders::ALL)
        .style(Style::default().fg(theme::ACTIVE_BORDER));

    let line = Line::from(vec![
        Span::styled(message, Style::default().fg(theme::TEXT)),
        Span::raw("  "),
        Span::styled("[Enter]", Style::default().fg(theme::ACTIVE_BORDER)),
        Span::styled(" confirm  ", Style::default().fg(theme::TEXT)),
        Span::styled("[Esc]", Style::default().fg(theme::ACTIVE_BORDER)),
        Span::styled(" cancel", Style::default().fg(theme::TEXT)),
    ]);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(Paragraph::new(line).block(block), popup_area);
}

fn centered_rect(width_percent: u16, area: Rect) -> Rect {
    let margin_h = (100 - width_percent) / 2;
    let vertical = Layout::vertical([
        Constraint::Percentage(45),
        Constraint::Length(3),
        Constraint::Percentage(45),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage(margin_h),
        Constraint::Percentage(width_percent),
        Constraint::Percentage(margin_h),
    ])
    .split(vertical[1])[1]
}
