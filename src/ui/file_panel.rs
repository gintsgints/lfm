use std::{
    io,
    path::{Path, PathBuf},
};

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::message::Message;
use crate::theme;

pub struct Entry {
    pub name: String,
    pub is_dir: bool,
}

pub struct Model {
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

    pub fn navigate_to(&mut self, path: PathBuf) {
        if let Ok(entries) = read_entries(&path) {
            self.current_dir = path;
            self.entries = entries;
            self.selection = 0;
        }
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
        Message::DirUp => {
            if let Some(parent) = model.current_dir.parent().map(Path::to_path_buf) {
                model.navigate_to(parent);
            }
        }
        Message::DirEnter => {
            if let Some(entry) = model.entries.get(model.selection)
                && entry.is_dir
            {
                let path = model.current_dir.join(&entry.name);
                model.navigate_to(path);
            }
        }
        _ => {}
    }
    model
}

pub fn render(frame: &mut Frame, area: Rect, model: &Model, active: bool) {
    let border_style = if active {
        Style::default().fg(theme::ACTIVE_BORDER)
    } else {
        Style::default().fg(theme::INACTIVE_BORDER)
    };

    let title = format!(" {} ", model.current_dir.display());
    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(theme::TEXT)))
        .borders(Borders::ALL)
        .style(border_style);

    let items: Vec<ListItem> = model
        .entries
        .iter()
        .map(|e| {
            if e.is_dir {
                ListItem::new(Span::styled(
                    format!("{}/", e.name),
                    Style::default().fg(theme::DIR_FG),
                ))
            } else {
                ListItem::new(Span::styled(
                    e.name.clone(),
                    Style::default().fg(theme::TEXT),
                ))
            }
        })
        .collect();

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(theme::HIGHLIGHT_BG)
            .fg(theme::HIGHLIGHT_FG)
            .add_modifier(Modifier::BOLD),
    );

    let mut state = ListState::default();
    if active {
        state.select(Some(model.selection));
    }

    frame.render_stateful_widget(list, area, &mut state);
}

fn read_entries(path: &Path) -> io::Result<Vec<Entry>> {
    let mut entries: Vec<Entry> = std::fs::read_dir(path)?
        .filter_map(std::result::Result::ok)
        .map(|e| Entry {
            name: e.file_name().to_string_lossy().into_owned(),
            is_dir: e.file_type().map(|t| t.is_dir()).unwrap_or(false),
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}
