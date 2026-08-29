use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::model::InputField;
use crate::theme;
use crate::ui::input_box;

/// Columns given to the mask field, label included. A narrow row gives it a
/// third of the width instead, so the query keeps the bulk of the room.
const MASK_WIDTH: u16 = 28;

/// Below this the row shows the query alone — a narrow terminal has no room to
/// split it in two.
const MIN_SPLIT_WIDTH: u16 = 60;

/// The input row shared by the content-search and file-find panels: the query
/// on the left, the file mask on the right.
///
/// `focused` says whether the row has the keys at all (Tab hands them to the
/// results); `field` says which of the two fields the cursor sits in.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    query: &input_box::Model,
    mask: &input_box::Model,
    focused: bool,
    field: InputField,
) {
    if area.width < MIN_SPLIT_WIDTH {
        render_query(frame, area, query, focused && field == InputField::Query);
        return;
    }

    let mask_width = MASK_WIDTH.min(area.width / 3);
    let cells =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(mask_width)]).split(area);
    render_query(
        frame,
        cells[0],
        query,
        focused && field == InputField::Query,
    );
    render_mask(frame, cells[1], mask, focused && field == InputField::Mask);
}

fn render_query(frame: &mut Frame, area: Rect, query: &input_box::Model, active: bool) {
    let prompt = Span::styled(
        if active { "> " } else { "  " },
        Style::default().fg(theme::active_border()),
    );
    let mut spans = vec![prompt];
    spans.extend(field_spans(query, active));
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_mask(frame: &mut Frame, area: Rect, mask: &input_box::Model, active: bool) {
    let label = Span::styled(
        "mask: ",
        Style::default().fg(if active {
            theme::active_border()
        } else {
            theme::inactive_border()
        }),
    );
    let mut spans = vec![label];
    spans.extend(field_spans(mask, active));
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// One field's text: bold with the cursor sitting _on_ a character while it has
/// the keys ("_" stands in at end-of-text), dim otherwise.
fn field_spans(model: &input_box::Model, active: bool) -> Vec<Span<'static>> {
    if !active {
        return vec![Span::styled(
            model.text.clone(),
            Style::default().fg(theme::inactive_border()),
        )];
    }

    model.cursor_spans(
        Style::default()
            .fg(theme::text())
            .add_modifier(Modifier::BOLD),
    )
}
