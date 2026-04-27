use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::theme;

pub fn render(frame: &mut Frame, area: Rect, message: &str) {
    let popup_area = centered_rect(65, 7, area);

    let block = Block::default()
        .title(Span::styled(
            " Error ",
            Style::default()
                .fg(Color::Black)
                .bg(theme::MOVE_TARGET_BORDER),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::MOVE_TARGET_BORDER));

    let text = Text::from(vec![
        Line::from(Span::styled(message, Style::default().fg(theme::TEXT))),
        Line::raw(""),
        Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(theme::ACTIVE_BORDER)),
            Span::styled(" / ", Style::default().fg(theme::INACTIVE_BORDER)),
            Span::styled("[Esc]", Style::default().fg(theme::ACTIVE_BORDER)),
            Span::styled("  dismiss", Style::default().fg(theme::TEXT)),
        ]),
    ]);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
        popup_area,
    );
}

fn centered_rect(width_percent: u16, height: u16, area: Rect) -> Rect {
    let margin_v = area.height.saturating_sub(height) / 2;
    let margin_h = (100 - width_percent) / 2;
    let vertical = Layout::vertical([
        Constraint::Length(margin_v),
        Constraint::Length(height),
        Constraint::Min(0),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage(margin_h),
        Constraint::Percentage(width_percent),
        Constraint::Percentage(margin_h),
    ])
    .split(vertical[1])[1]
}
