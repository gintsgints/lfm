use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, Paragraph},
};
use ratatui_image::{StatefulImage, thread::ThreadProtocol};
use tui_view::TuiView;

use crate::image_view::ImageView;
use crate::model::{FileView, ViewContent};
use crate::theme;

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

    match &mut view.content {
        ViewContent::Text(state) => {
            frame.render_stateful_widget(TuiView::new().block(block), area, state);
        }
        ViewContent::Image(view) => {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            render_image(frame, inner, view);
        }
    }
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
