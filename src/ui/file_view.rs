use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Span,
    widgets::{Block, Borders, Clear},
};
use tui_view::TuiView;

use crate::model::FileView;
use crate::theme;

pub fn render(frame: &mut Frame, area: Rect, view: &mut FileView) {
    let popup_area = centered_rect(90, 90, area);

    let block = Block::default()
        .title(Span::styled(
            format!(" {} — {} ", view.name, view.state.view().name()),
            Style::default().fg(theme::active_border()),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::popup_border()));

    frame.render_widget(Clear, popup_area);
    frame.render_stateful_widget(TuiView::new().block(block), popup_area, &mut view.state);
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
