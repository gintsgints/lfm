use std::{
    io,
    path::{Path, PathBuf},
};

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::message::Message;

pub struct Entry {
    pub name: String,
    pub is_dir: bool,
}

pub struct Model {
    #[allow(dead_code)]
    pub current_dir: PathBuf,
    pub entries: Vec<Entry>,
    pub selection: usize,
}

impl Model {
    pub fn init() -> io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let entries = read_entries(&current_dir)?;
        Ok(Self {
            current_dir,
            entries,
            selection: 0,
        })
    }

    fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

pub fn update(mut model: Model, msg: Message) -> Model {
    match msg {
        Message::SelectUp => {
            model.selection = model.selection.saturating_sub(1);
        }
        Message::SelectDown => {
            let count = model.entry_count();
            if count > 0 {
                model.selection = (model.selection + 1).min(count - 1);
            }
        }
        _ => {}
    }
    model
}

pub fn render(frame: &mut Frame, area: Rect, model: &Model, active: bool) {
    let border_style = if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .style(border_style);

    let items: Vec<ListItem> = model
        .entries
        .iter()
        .map(|e| {
            let name = if e.is_dir {
                format!("{}/", e.name)
            } else {
                e.name.clone()
            };
            ListItem::new(name)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    state.select(Some(model.selection));

    frame.render_stateful_widget(list, area, &mut state);
}

fn read_entries(path: &Path) -> io::Result<Vec<Entry>> {
    let mut entries: Vec<Entry> = std::fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .map(|e| Entry {
            name: e.file_name().to_string_lossy().into_owned(),
            is_dir: e.file_type().map(|t| t.is_dir()).unwrap_or(false),
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}
