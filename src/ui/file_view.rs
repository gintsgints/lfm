use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_image::{StatefulImage, thread::ThreadProtocol};
use tui_view::TuiView;

use crate::image_view::ImageView;
use crate::model::{FileView, ViewContent};
use crate::theme;
use crate::view_search::{self, ViewSearch};

/// Render the viewer as the right-hand panel. `focused` follows Tab and picks
/// the border colour the same way the file panels do.
pub fn render(frame: &mut Frame, area: Rect, view: &mut FileView, focused: bool) {
    let border = if focused {
        theme::active_border()
    } else {
        theme::inactive_border()
    };

    let block = Block::default()
        .title(Span::styled(
            format!(" {} — {} ", view.name, view.content.kind_name()),
            Style::default().fg(border),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border));

    // The block is drawn on its own rather than handed to the widget, so the
    // search row can take the bottom strip of the area inside its borders.
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let (body, search_area) = split_for_search(inner, view);

    // Remember the body size: the search needs the width to place its matches,
    // and the height to tell whether one is already on screen.
    view.viewport_width = body.width;
    view.viewport_height = body.height;
    // A resize moves every wrapped row, so the matches are found again at the
    // width they will be drawn at.
    if view.search.width != body.width {
        view_search::recompute(&mut view.search, &view.content, body.width);
    }

    match &mut view.content {
        ViewContent::Text(state) => {
            frame.render_stateful_widget(TuiView::new(), body, state);
        }
        ViewContent::Image(image) => render_image(frame, body, image),
    }

    highlight_matches(frame, body, view);
    if let Some(area) = search_area {
        render_search_row(frame, area, &view.search);
    }
}

/// Split the panel's inside into the text body and the search row, when there
/// is a search to show and a row to spare for it.
fn split_for_search(inner: Rect, view: &FileView) -> (Rect, Option<Rect>) {
    if !view.search.is_visible() || inner.height < 2 {
        return (inner, None);
    }
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(inner);
    (rows[0], Some(rows[1]))
}

/// Mark the matches that fall on screen, over the text the widget has already
/// drawn: the current match in the highlight colours, the rest underlined so
/// the view's own syntax colours survive.
fn highlight_matches(frame: &mut Frame, body: Rect, view: &FileView) {
    let ViewContent::Text(state) = &view.content else {
        return;
    };
    if view.search.matches.is_empty() {
        return;
    }
    let scroll = state.scroll();
    let height = body.height as usize;
    let current = Style::default()
        .bg(theme::highlight_bg())
        .fg(theme::highlight_fg())
        .add_modifier(Modifier::BOLD);
    let other = Style::default().add_modifier(Modifier::UNDERLINED | Modifier::BOLD);

    let buf = frame.buffer_mut();
    for (i, m) in view.search.matches.iter().enumerate() {
        if m.row < scroll || m.row >= scroll + height {
            continue;
        }
        let Ok(offset) = u16::try_from(m.row - scroll) else {
            continue;
        };
        let style = if i == view.search.current {
            current
        } else {
            other
        };
        let y = body.y + offset;
        let start = body.x.saturating_add(m.col);
        let end = start.saturating_add(m.width).min(body.right());
        for x in start..end {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_style(style);
            }
        }
    }
}

/// The `/` row at the bottom of the panel: the query, and where the viewer is
/// among the matches it found.
fn render_search_row(frame: &mut Frame, area: Rect, search: &ViewSearch) {
    let text_style = Style::default()
        .fg(theme::text())
        .add_modifier(Modifier::BOLD);
    let mut spans = vec![Span::styled(
        "/",
        Style::default().fg(theme::active_border()),
    )];
    if search.input.active {
        spans.extend(search.input.cursor_spans(text_style));
    } else {
        spans.push(Span::styled(search.query.clone(), text_style));
    }
    if let Some(status) = match_status(search) {
        spans.push(Span::styled(
            status,
            Style::default().fg(theme::inactive_border()),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// How far through the matches the viewer is, or why there is nothing to step
/// through. Nothing is said while the query is still being typed.
fn match_status(search: &ViewSearch) -> Option<String> {
    if search.input.active || search.query.is_empty() {
        return None;
    }
    if search.matches.is_empty() {
        return Some("  no match".to_owned());
    }
    Some(format!("  {}/{}", search.current + 1, search.matches.len()))
}

/// Draw the image inside the panel's borders.
///
/// The widget only draws what the worker has already encoded: while a decode or
/// a resize is outstanding it draws nothing, so the panel says so instead of
/// going blank. Rendering never resizes or encodes on this thread — it only
/// posts the request the worker picks up.
fn render_image(frame: &mut Frame, area: Rect, view: &mut ImageView) {
    let status = match &view.error {
        Some(err) => Some(format!("<image error: {err}>")),
        None if !view.is_decoded() => Some("<decoding…>".to_owned()),
        None => None,
    };
    if let Some(status) = status {
        frame.render_widget(
            Paragraph::new(status).style(Style::default().fg(theme::text())),
            area,
        );
        return;
    }
    frame.render_stateful_widget(
        StatefulImage::<ThreadProtocol>::default(),
        area,
        &mut view.protocol,
    );
}
