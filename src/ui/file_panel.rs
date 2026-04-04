use std::{
    collections::BTreeSet,
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
use crate::ui::{input_box, search_box};

pub struct DeleteTarget {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

pub struct Entry {
    pub name: String,
    pub is_dir: bool,
}

pub struct Model {
    pub current_dir: PathBuf,
    pub entries: Vec<Entry>,
    pub selection: usize,
    pub selected: BTreeSet<usize>,
    pub search: search_box::Model,
    pub new_path_input: input_box::Model,
    pub delete_confirm: bool,
    pub delete_targets: Vec<DeleteTarget>,
}

impl Model {
    pub fn init() -> io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let entries = read_entries(&current_dir)?;
        Ok(Self {
            current_dir,
            entries,
            selection: 0,
            selected: BTreeSet::new(),
            search: search_box::Model::new(),
            new_path_input: input_box::Model::new(),
            delete_confirm: false,
            delete_targets: Vec::new(),
        })
    }

    fn entry_count(&self) -> usize {
        self.visible_entries().count()
    }

    pub fn navigate_to(&mut self, path: PathBuf) {
        if let Ok(entries) = read_entries(&path) {
            self.current_dir = path;
            self.entries = entries;
            self.selection = 0;
            self.selected.clear();
            self.search.clear();
            self.new_path_input.close();
            self.delete_confirm = false;
            self.delete_targets.clear();
        }
    }

    pub fn action_targets(&self) -> Vec<DeleteTarget> {
        if self.selected.is_empty() {
            self.visible_entries()
                .nth(self.selection)
                .map(|(_, e)| DeleteTarget {
                    name: e.name.clone(),
                    path: self.current_dir.join(&e.name),
                    is_dir: e.is_dir,
                })
                .into_iter()
                .collect()
        } else {
            self.visible_entries()
                .filter(|(i, _)| self.selected.contains(i))
                .map(|(_, e)| DeleteTarget {
                    name: e.name.clone(),
                    path: self.current_dir.join(&e.name),
                    is_dir: e.is_dir,
                })
                .collect()
        }
    }

    fn toggle_selected(&mut self, index: usize) {
        if !self.selected.remove(&index) {
            self.selected.insert(index);
        }
    }

    fn visible_entries(&self) -> impl Iterator<Item = (usize, &Entry)> {
        let filter = self.search.text.to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter(move |(_, e)| filter.is_empty() || e.name.to_lowercase().contains(&filter))
    }
}

pub fn update(mut model: Model, msg: Message) -> Model {
    match msg {
        Message::EnterFilter
        | Message::FilterChar(_)
        | Message::FilterBackspace
        | Message::ConfirmFilter
        | Message::ExitFilter => {
            let raw_idx = model.visible_entries().nth(model.selection).map(|(i, _)| i);
            let (search, reset) = search_box::update(model.search, msg);
            model.search = search;
            if reset {
                model.selection = raw_idx.unwrap_or(0);
            }
        }
        Message::NewPath
        | Message::NewPathChar(_)
        | Message::NewPathBackspace
        | Message::NewPathCancel
        | Message::NewPathConfirm => {
            model = update_new_path(model, msg);
        }
        Message::DeleteFiles | Message::DeleteCancel | Message::DeleteConfirm => {
            model = update_delete(model, msg);
        }
        Message::SelectUp => {
            model.selection = model.selection.saturating_sub(1);
        }
        Message::SelectDown => {
            let count = model.entry_count();
            if count > 0 {
                model.selection = (model.selection + 1).min(count - 1);
            }
        }
        Message::MarkSelectUp => {
            model.toggle_selected(model.selection);
            model.selection = model.selection.saturating_sub(1);
        }
        Message::MarkSelectDown => {
            model.toggle_selected(model.selection);
            let count = model.entry_count();
            if count > 0 {
                model.selection = (model.selection + 1).min(count - 1);
            }
        }
        Message::ClearSelection => {
            model.selected.clear();
        }
        Message::DirUp => {
            if let Some(parent) = model.current_dir.parent().map(Path::to_path_buf) {
                let came_from = model
                    .current_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned());
                model.navigate_to(parent);
                if let Some(name) = came_from
                    && let Some(idx) = model.entries.iter().position(|e| e.name == name)
                {
                    model.selection = idx;
                }
            }
        }
        Message::DirEnter => {
            let target = model
                .visible_entries()
                .nth(model.selection)
                .filter(|(_, e)| e.is_dir)
                .map(|(_, e)| model.current_dir.join(&e.name));
            if let Some(path) = target {
                model.navigate_to(path);
            }
        }
        _ => {}
    }
    model
}

fn update_new_path(mut model: Model, msg: Message) -> Model {
    match msg {
        Message::NewPath => {
            model.new_path_input.open();
        }
        Message::NewPathChar(c) => {
            let (input, _) = input_box::update(model.new_path_input, Some(c), false, false, false);
            model.new_path_input = input;
        }
        Message::NewPathBackspace => {
            let (input, _) = input_box::update(model.new_path_input, None, true, false, false);
            model.new_path_input = input;
        }
        Message::NewPathCancel => {
            model.new_path_input.close();
        }
        Message::NewPathConfirm => {
            let text = model.new_path_input.text.clone();
            model.new_path_input.close();
            if !text.is_empty() {
                let target = model.current_dir.join(&text);
                let created = if text.ends_with('/') {
                    std::fs::create_dir_all(&target)
                } else {
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent).ok();
                    }
                    std::fs::File::create(&target).map(|_| ())
                };
                if created.is_ok() {
                    let dir = model.current_dir.clone();
                    model.navigate_to(dir);
                }
            }
        }
        _ => {}
    }
    model
}

fn update_delete(mut model: Model, msg: Message) -> Model {
    match msg {
        Message::DeleteFiles => {
            let targets = model.action_targets();
            if !targets.is_empty() {
                model.delete_targets = targets;
                model.delete_confirm = true;
            }
        }
        Message::DeleteCancel => {
            model.delete_confirm = false;
            model.delete_targets.clear();
        }
        Message::DeleteConfirm => {
            model.delete_confirm = false;
            for target in &model.delete_targets {
                if target.is_dir {
                    std::fs::remove_dir_all(&target.path).ok();
                } else {
                    std::fs::remove_file(&target.path).ok();
                }
            }
            model.delete_targets.clear();
            model.selected.clear();
            let dir = model.current_dir.clone();
            model.navigate_to(dir);
        }
        _ => {}
    }
    model
}

pub fn render(frame: &mut Frame, area: Rect, model: &Model, active: bool, is_copy_target: bool) {
    let border_style = if is_copy_target {
        Style::default().fg(theme::COPY_TARGET_BORDER)
    } else if active {
        Style::default().fg(theme::ACTIVE_BORDER)
    } else {
        Style::default().fg(theme::INACTIVE_BORDER)
    };

    let path_label = if is_copy_target {
        format!("→  {}", model.current_dir.display())
    } else {
        model.current_dir.display().to_string()
    };
    let title = search_box::title(&model.search, &path_label);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(border_style);

    let items: Vec<ListItem> = model
        .visible_entries()
        .map(|(i, e)| {
            let is_selected = model.selected.contains(&i);
            let fg = if is_selected {
                theme::SELECTED_FG
            } else if e.is_dir {
                theme::DIR_FG
            } else {
                theme::TEXT
            };
            let name = if e.is_dir {
                format!("{}/", e.name)
            } else {
                e.name.clone()
            };
            ListItem::new(Span::styled(name, Style::default().fg(fg)))
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
