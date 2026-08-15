use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, Paragraph},
};
use ratatui_image::{StatefulImage, protocol::StatefulProtocol};
use tui_view::TuiView;

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
        ViewContent::Image(protocol) => {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            render_image(frame, inner, protocol);
        }
    }
}

/// Draw the image inside the panel's borders. Encoding happens during the
/// render and can fail (an unsupported protocol, a terminal that rejected the
/// sequence), so the error is reported in place of the image on the next frame.
fn render_image(frame: &mut Frame, area: Rect, protocol: &mut StatefulProtocol) {
    frame.render_stateful_widget(StatefulImage::default(), area, protocol);
    if let Some(Err(err)) = protocol.last_encoding_result() {
        frame.render_widget(
            Paragraph::new(format!("<image error: {err}>"))
                .style(Style::default().fg(theme::text())),
            area,
        );
    }
}
