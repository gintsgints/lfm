use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::model::FileView;
use crate::theme;

pub fn render(frame: &mut Frame, area: Rect, view: &FileView) {
    let popup_area = centered_rect(90, 90, area);

    let key = |s: &'static str| Span::styled(s, Style::default().fg(theme::ACTIVE_BORDER));
    let dim = |s: &'static str| Span::styled(s, Style::default().fg(theme::INACTIVE_BORDER));
    let bottom = Line::from(vec![
        key("[j/k]"),
        dim(" scroll  "),
        key("[PgUp/PgDn]"),
        dim(" page  "),
        key("[Esc/q]"),
        dim(" close "),
    ]);

    let block = Block::default()
        .title(Span::styled(
            format!(" {} ", view.name),
            Style::default().fg(theme::ACTIVE_BORDER),
        ))
        .title_bottom(bottom)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::POPUP_BORDER));

    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let lines: Vec<Line> = view
        .content
        .lines()
        .map(|l| Line::from(Span::styled(l.to_owned(), Style::default().fg(theme::TEXT))))
        .collect();

    let chunks = Layout::vertical([Constraint::Min(0)]).split(inner);
    frame.render_widget(Paragraph::new(lines).scroll((view.scroll, 0)), chunks[0]);
}

/// Total number of text rows in the file, used by update logic to clamp
/// [`FileView::scroll`].
pub fn line_count(view: &FileView) -> u16 {
    u16::try_from(view.content.lines().count()).unwrap_or(u16::MAX)
}

fn centered_rect(width_percent: u16, height_percent: u16, area: Rect) -> Rect {
    let margin_h = (100 - width_percent) / 2;
    let margin_v = (100 - height_percent) / 2;
    let vertical = Layout::vertical([
        Constraint::Percentage(margin_v),
        Constraint::Percentage(height_percent),
        Constraint::Percentage(margin_v),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage(margin_h),
        Constraint::Percentage(width_percent),
        Constraint::Percentage(margin_h),
    ])
    .split(vertical[1])[1]
}
