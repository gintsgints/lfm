use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::model::CaptureView;
use crate::theme;

/// Render the captured output as its own full-screen view: the file panels are
/// not drawn behind it, so long output never lands on top of panel rows.
pub fn render(frame: &mut Frame, area: Rect, view: &CaptureView) {
    let header_fg = match view.exit_code {
        Some(0) => theme::active_border(),
        Some(_) | None => theme::move_target_border(),
    };
    let header = match view.exit_code {
        Some(code) => format!(" exit {code} — {} ", view.label),
        None => format!(" spawn failed — {} ", view.label),
    };

    let block = Block::default()
        .title(Span::styled(header, Style::default().fg(header_fg)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::popup_border()));

    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = body(view)
        .lines()
        .map(|l| {
            Line::from(Span::styled(
                l.to_owned(),
                Style::default().fg(theme::text()),
            ))
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((view.scroll, 0)),
        inner,
    );
}

fn body(view: &CaptureView) -> &str {
    if view.output.is_empty() {
        "(no output)"
    } else {
        view.output.as_str()
    }
}

/// Total scrollable rows in the body, used by update logic to clamp
/// [`CaptureView::scroll`].
pub fn line_count(view: &CaptureView) -> u16 {
    u16::try_from(body(view).lines().count()).unwrap_or(u16::MAX)
}
