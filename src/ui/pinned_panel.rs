use std::path::PathBuf;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};

use crate::theme;

use crate::message::Message;

pub struct Model {
    pub pins: Vec<PathBuf>,
    pub selection: usize,
}

impl Model {
    pub fn new() -> Self {
        Self {
            pins: Vec::new(),
            selection: 0,
        }
    }

    fn pin_count(&self) -> usize {
        self.pins.len()
    }
}

pub fn update(mut model: Model, msg: Message) -> Model {
    match msg {
        Message::SelectUp => {
            model.selection = model.selection.saturating_sub(1);
        }
        Message::SelectDown => {
            let count = model.pin_count();
            if count > 0 {
                model.selection = (model.selection + 1).min(count - 1);
            }
        }
        _ => {}
    }
    model
}

pub fn render(frame: &mut Frame, area: Rect, model: &Model) {
    let popup_area = centered_rect(50, 60, area);

    let block = Block::default()
        .title(Span::styled(
            " Pinned Directories  [p] pin  [Enter/Space] go  [d] delete  [Esc] close ",
            Style::default().fg(theme::TEXT),
        ))
        .borders(Borders::ALL)
        .style(Style::default().fg(theme::POPUP_BORDER));

    let items: Vec<ListItem> = model
        .pins
        .iter()
        .map(|p| {
            ListItem::new(Span::styled(
                p.display().to_string(),
                Style::default().fg(theme::DIR_FG),
            ))
        })
        .collect();

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(theme::HIGHLIGHT_BG)
            .fg(theme::HIGHLIGHT_FG)
            .add_modifier(Modifier::BOLD),
    );

    let mut state = ListState::default();
    if !model.pins.is_empty() {
        state.select(Some(model.selection));
    }

    frame.render_widget(Clear, popup_area);
    frame.render_stateful_widget(list, popup_area, &mut state);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let margin_v = (100 - percent_y) / 2;
    let margin_h = (100 - percent_x) / 2;

    let vertical = Layout::vertical([
        Constraint::Percentage(margin_v),
        Constraint::Percentage(percent_y),
        Constraint::Percentage(margin_v),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage(margin_h),
        Constraint::Percentage(percent_x),
        Constraint::Percentage(margin_h),
    ])
    .split(vertical[1])[1]
}
