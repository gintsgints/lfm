use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Span,
    widgets::{Block, Borders, Clear, Gauge},
};

use crate::{
    model::{TransferOp, TransferProgress},
    theme,
};

pub fn render(frame: &mut Frame, area: Rect, progress: &TransferProgress) {
    let popup_area = centered_rect(60, area);

    #[allow(clippy::cast_precision_loss)]
    let ratio = if progress.total == 0 {
        0.0
    } else {
        (progress.current as f64 / progress.total as f64).clamp(0.0, 1.0)
    };

    let title = match progress.op {
        TransferOp::Copy => " Copying ",
        TransferOp::Move => " Moving ",
        TransferOp::Delete => " Deleting ",
    };
    let gauge_color = match progress.op {
        TransferOp::Copy => theme::COPY_TARGET_BORDER,
        TransferOp::Move | TransferOp::Delete => theme::MOVE_TARGET_BORDER,
    };

    let label = format!("{} / {}", progress.current, progress.total);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(theme::ACTIVE_BORDER));

    let gauge = Gauge::default()
        .block(block)
        .gauge_style(Style::default().fg(gauge_color))
        .ratio(ratio)
        .label(Span::raw(label));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(gauge, popup_area);
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
