use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, PartialEq)]
pub enum ActivePanel {
    Dirs,
    Files,
    Command,
}

impl ActivePanel {
    pub fn next(self) -> Self {
        match self {
            ActivePanel::Dirs => ActivePanel::Files,
            ActivePanel::Files => ActivePanel::Command,
            ActivePanel::Command => ActivePanel::Dirs,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            ActivePanel::Dirs => ActivePanel::Command,
            ActivePanel::Files => ActivePanel::Dirs,
            ActivePanel::Command => ActivePanel::Files,
        }
    }
}

pub struct Entry {
    pub name: String,
    pub is_dir: bool,
}

pub struct App {
    #[allow(dead_code)]
    pub current_dir: PathBuf,
    pub entries: Vec<Entry>,
    pub active_panel: ActivePanel,
    pub dir_selection: usize,
    pub file_selection: usize,
}

impl App {
    pub fn new() -> io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let entries = read_entries(&current_dir)?;
        Ok(Self {
            current_dir,
            entries,
            active_panel: ActivePanel::Dirs,
            dir_selection: 0,
            file_selection: 0,
        })
    }

    pub fn dirs(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter().filter(|e| e.is_dir)
    }

    pub fn files(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter().filter(|e| !e.is_dir)
    }

    pub fn dir_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_dir).count()
    }

    pub fn file_count(&self) -> usize {
        self.entries.iter().filter(|e| !e.is_dir).count()
    }

    pub fn select_up(&mut self) {
        match self.active_panel {
            ActivePanel::Dirs => self.dir_selection = self.dir_selection.saturating_sub(1),
            ActivePanel::Files => self.file_selection = self.file_selection.saturating_sub(1),
            ActivePanel::Command => {}
        }
    }

    pub fn select_down(&mut self) {
        match self.active_panel {
            ActivePanel::Dirs => {
                let count = self.dir_count();
                if count > 0 {
                    self.dir_selection = (self.dir_selection + 1).min(count - 1);
                }
            }
            ActivePanel::Files => {
                let count = self.file_count();
                if count > 0 {
                    self.file_selection = (self.file_selection + 1).min(count - 1);
                }
            }
            ActivePanel::Command => {}
        }
    }
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
