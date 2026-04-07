use std::{
    collections::BTreeSet,
    io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::archive;
#[cfg(feature = "debug")]
use crate::debug_log;
use crate::message::Message;
use crate::theme;
use crate::ui::{input_box, search_box};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum SortOrder {
    #[default]
    Name,
    Modified,
    Extension,
    Size,
}

impl SortOrder {
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Modified,
            Self::Modified => Self::Extension,
            Self::Extension => Self::Size,
            Self::Size => Self::Name,
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Modified => "date",
            Self::Extension => "ext",
            Self::Size => "size",
        }
    }
}

pub struct DeleteTarget {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

pub struct Model {
    pub current_dir: PathBuf,
    pub entries: Vec<Entry>,
    pub selection: usize,
    pub selected: BTreeSet<usize>,
    pub search: search_box::Model,
    pub new_path_input: input_box::Model,
    pub goto_input: input_box::Model,
    pub delete_confirm: bool,
    pub delete_targets: Vec<DeleteTarget>,
    pub sort_order: SortOrder,
}

impl Model {
    pub fn init() -> io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let sort_order = SortOrder::default();
        let mut entries = read_entries(&current_dir)?;
        sort_entries(&mut entries, sort_order);
        Ok(Self {
            current_dir,
            entries,
            selection: 0,
            selected: BTreeSet::new(),
            search: search_box::Model::new(),
            new_path_input: input_box::Model::new(),
            goto_input: input_box::Model::new(),
            delete_confirm: false,
            delete_targets: Vec::new(),
            sort_order,
        })
    }

    fn entry_count(&self) -> usize {
        self.visible_entries().count()
    }

    pub fn navigate_to(&mut self, path: PathBuf) {
        if let Ok(mut entries) = read_entries(&path) {
            sort_entries(&mut entries, self.sort_order);
            self.current_dir = path;
            self.entries = entries;
            self.selection = 0;
            self.selected.clear();
            self.search.clear();
            self.new_path_input.close();
            self.goto_input.close();
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

    pub fn visible_entries(&self) -> impl Iterator<Item = (usize, &Entry)> {
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
                // Find the visual position of the previously selected item in the new
                // filtered view. Falls back to 0 if the item is no longer visible.
                model.selection = raw_idx
                    .and_then(|ri| model.visible_entries().position(|(i, _)| i == ri))
                    .unwrap_or(0);
            }
        }
        Message::NewPath
        | Message::NewPathChar(_)
        | Message::NewPathBackspace
        | Message::NewPathCancel
        | Message::NewPathConfirm => {
            model = update_new_path(model, msg);
        }
        Message::GotoPath
        | Message::GotoPathChar(_)
        | Message::GotoPathBackspace
        | Message::GotoPathCancel
        | Message::GotoPathConfirm => {
            model = update_goto(model, msg);
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
        Message::CycleSort => {
            model.sort_order = model.sort_order.next();
            sort_entries(&mut model.entries, model.sort_order);
            model.selection = 0;
            model.selected.clear();
        }
        Message::ZipFiles | Message::UnzipFile => {
            model = update_archive(model, msg);
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
            model.new_path_input.insert(c);
        }
        Message::NewPathBackspace => {
            model.new_path_input.backspace();
        }
        Message::NewPathCursorLeft => {
            model.new_path_input.move_left();
        }
        Message::NewPathCursorRight => {
            model.new_path_input.move_right();
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

fn update_goto(mut model: Model, msg: Message) -> Model {
    match msg {
        Message::GotoPath => {
            model.goto_input.open();
        }
        Message::GotoPathChar(c) => {
            model.goto_input.insert(c);
        }
        Message::GotoPathBackspace => {
            model.goto_input.backspace();
        }
        Message::GotoPathCursorLeft => {
            model.goto_input.move_left();
        }
        Message::GotoPathCursorRight => {
            model.goto_input.move_right();
        }
        Message::GotoPathCancel => {
            model.goto_input.close();
        }
        Message::GotoPathConfirm => {
            let text = model.goto_input.text.clone();
            model.goto_input.close();
            if !text.is_empty() {
                let expanded = if let Some(rest) = text.strip_prefix("~/") {
                    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(rest))
                } else if text == "~" {
                    std::env::var_os("HOME").map(PathBuf::from)
                } else {
                    None
                };
                let target = expanded.unwrap_or_else(|| {
                    let p = PathBuf::from(&text);
                    if p.is_absolute() {
                        p
                    } else {
                        model.current_dir.join(p)
                    }
                });
                if target.is_dir() {
                    model.navigate_to(target);
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

fn update_archive(mut model: Model, msg: Message) -> Model {
    match msg {
        Message::ZipFiles => {
            let targets = model.action_targets();
            if !targets.is_empty() {
                let first_name = &targets[0].name;
                let stem = if targets[0].is_dir {
                    first_name.as_str()
                } else {
                    Path::new(first_name)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(first_name.as_str())
                };
                let archive_name = if targets.len() == 1 {
                    format!("{stem}.zip")
                } else {
                    "archive.zip".to_owned()
                };
                let dest = model.current_dir.join(&archive_name);
                let sources: Vec<PathBuf> = targets.iter().map(|t| t.path.clone()).collect();
                if archive::zip_paths(&sources, &dest).is_ok() {
                    model.selected.clear();
                    let dir = model.current_dir.clone();
                    model.navigate_to(dir);
                }
            }
        }
        Message::UnzipFile => {
            let name = model
                .visible_entries()
                .nth(model.selection)
                .map(|(_, e)| e.name.clone());
            if let Some(name) = name {
                let src = model.current_dir.join(&name);
                let ext = Path::new(&name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let result = if ext == "zip" {
                    let dest_name = name.trim_end_matches(".zip");
                    let dest = model.current_dir.join(dest_name);
                    std::fs::create_dir_all(&dest).ok();
                    archive::unzip(&src, &dest)
                } else if name.to_ascii_lowercase().ends_with(".tar.gz") {
                    let dest_name = &name[..name.len() - ".tar.gz".len()];
                    let dest = model.current_dir.join(dest_name);
                    std::fs::create_dir_all(&dest).ok();
                    archive::untar_gz(&src, &dest)
                } else {
                    return model;
                };
                if result.is_ok() {
                    let dir = model.current_dir.clone();
                    model.navigate_to(dir);
                }
            }
        }
        _ => {}
    }
    model
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    model: &Model,
    active: bool,
    is_copy_target: bool,
    is_move_target: bool,
) {
    let border_style = if is_move_target {
        Style::default().fg(theme::MOVE_TARGET_BORDER)
    } else if is_copy_target {
        Style::default().fg(theme::COPY_TARGET_BORDER)
    } else if active {
        Style::default().fg(theme::ACTIVE_BORDER)
    } else {
        Style::default().fg(theme::INACTIVE_BORDER)
    };

    let path_label = if is_move_target {
        format!(
            "↝  {} [Sorted by: {}]",
            model.current_dir.display(),
            model.sort_order.label()
        )
    } else if is_copy_target {
        format!(
            "→  {} [Sorted by: {}]",
            model.current_dir.display(),
            model.sort_order.label()
        )
    } else {
        format!(
            "{} [Sorted by: {}]",
            model.current_dir.display(),
            model.sort_order.label()
        )
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
                format!("󰉋 {}/", e.name)
            } else {
                format!("󰈙 {}", e.name)
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
    #[cfg(feature = "debug")]
    debug_log!("Read entries from path: {path:?}");
    let entries: Vec<Entry> = std::fs::read_dir(path)?
        .filter_map(std::result::Result::ok)
        .map(|e| {
            let meta = e.metadata().ok();
            Entry {
                name: e.file_name().to_string_lossy().into_owned(),
                is_dir: e.file_type().map(|t| t.is_dir()).unwrap_or(false),
                size: meta.as_ref().map_or(0, std::fs::Metadata::len),
                modified: meta.and_then(|m| m.modified().ok()),
            }
        })
        .collect();
    Ok(entries)
}

fn sort_entries(entries: &mut [Entry], order: SortOrder) {
    match order {
        SortOrder::Name => entries.sort_by(|a, b| a.name.cmp(&b.name)),
        SortOrder::Modified => entries.sort_by(|a, b| b.modified.cmp(&a.modified)),
        SortOrder::Extension => entries.sort_by(|a, b| {
            let ext_a = Path::new(&a.name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let ext_b = Path::new(&b.name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            ext_a.cmp(ext_b).then_with(|| a.name.cmp(&b.name))
        }),
        SortOrder::Size => entries.sort_by(|a, b| b.size.cmp(&a.size)),
    }
}
