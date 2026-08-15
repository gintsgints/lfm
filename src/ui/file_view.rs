use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders},
};
use tui_view::TuiView;

use crate::model::FileView;
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
            format!(" {} — {} ", view.name, view.state.view().name()),
            Style::default().fg(border),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border));

    frame.render_stateful_widget(TuiView::new().block(block), area, &mut view.state);
}
