use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::model::CapturePopup;
use crate::theme;

pub fn render(frame: &mut Frame, area: Rect, popup: &CapturePopup) {
    let popup_area = centered_rect(90, 90, area);

    let header_fg = match popup.exit_code {
        Some(0) => theme::active_border(),
        Some(_) | None => theme::move_target_border(),
    };
    let header = match popup.exit_code {
        Some(code) => format!(" exit {code} — {} ", popup.label),
        None => format!(" spawn failed — {} ", popup.label),
    };

    let key = |s: &'static str| Span::styled(s, Style::default().fg(theme::active_border()));
    let dim = |s: &'static str| Span::styled(s, Style::default().fg(theme::inactive_border()));
    let bottom = Line::from(vec![
        key("[j/k]"),
        dim(" scroll  "),
        key("[PgUp/PgDn]"),
        dim(" page  "),
        key("[Esc/Enter]"),
        dim(" close "),
    ]);

    let block = Block::default()
        .title(Span::styled(header, Style::default().fg(header_fg)))
        .title_bottom(bottom)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::popup_border()));

    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let body = if popup.output.is_empty() {
        "(no output)".to_owned()
    } else {
        popup.output.clone()
    };
    let lines: Vec<Line> = body
        .lines()
        .map(|l| {
            Line::from(Span::styled(
                l.to_owned(),
                Style::default().fg(theme::text()),
            ))
        })
        .collect();

    let chunks = Layout::vertical([Constraint::Min(0)]).split(inner);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((popup.scroll, 0)),
        chunks[0],
    );
}

/// Total scrollable rows in the body for the given popup area, used by update
/// logic to clamp [`CapturePopup::scroll`].
pub fn line_count(popup: &CapturePopup) -> u16 {
    let body = if popup.output.is_empty() {
        "(no output)"
    } else {
        popup.output.as_str()
    };
    u16::try_from(body.lines().count()).unwrap_or(u16::MAX)
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
