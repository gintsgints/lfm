use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::theme;
use crate::ui::input_box;

/// The filter bar above a file panel. The field itself is a plain
/// `input_box::Model` owned by the panel; this module only draws it.
pub fn render(frame: &mut Frame, area: Rect, model: &input_box::Model) {
    let border_style = if model.active {
        Style::default().fg(theme::active_border())
    } else {
        Style::default().fg(theme::inactive_border())
    };

    let text_style = Style::default()
        .fg(theme::text())
        .add_modifier(Modifier::BOLD);

    let content = if model.active {
        Line::from(model.cursor_spans(text_style))
    } else {
        Line::from(Span::styled(model.text.clone(), text_style))
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(" filter ", Style::default().fg(theme::text())));

    frame.render_widget(Paragraph::new(content).block(block), area);
}

pub fn title(path_label: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {path_label} "),
        Style::default().fg(theme::text()),
    ))
}
