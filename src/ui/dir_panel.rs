use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::app::Entry;

pub fn render<'a>(
    frame: &mut Frame,
    area: Rect,
    dirs: impl Iterator<Item = &'a Entry>,
    active: bool,
    selected: usize,
) {
    let border_style = if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let block = Block::default()
        .title(" Directories ")
        .borders(Borders::ALL)
        .style(border_style);

    let items: Vec<ListItem> = dirs.map(|e| ListItem::new(e.name.clone())).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    state.select(Some(selected));

    frame.render_stateful_widget(list, area, &mut state);
}
