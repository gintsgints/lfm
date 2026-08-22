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
pub fn render(frame: &mut Frame, area: Rect, view: &mut CaptureView) {
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

    // Remember the body size so the update logic can clamp scrolling to the
    // wrapped row count rather than guessing.
    view.viewport_width = inner.width;
    view.viewport_height = inner.height;
    view.scroll = view.scroll.min(max_scroll(view));

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

/// Rows the wrapped body occupies at the last rendered width.
fn row_count(view: &CaptureView) -> u16 {
    let width = usize::from(view.viewport_width);
    let rows: usize = body(view)
        .lines()
        .map(|l| {
            if width == 0 {
                return 1;
            }
            Line::raw(l).width().max(1).div_ceil(width)
        })
        .sum();
    u16::try_from(rows).unwrap_or(u16::MAX)
}

/// Largest scroll offset that still keeps the last row on screen.
pub fn max_scroll(view: &CaptureView) -> u16 {
    row_count(view).saturating_sub(view.viewport_height)
}

/// How far PgUp/PgDn move — one screenful, minus a row of overlap.
pub fn page_step(view: &CaptureView) -> u16 {
    view.viewport_height.saturating_sub(1).max(1)
}
