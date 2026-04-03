use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, PartialEq)]
pub enum ActivePanel {
    Files,
    Command,
}

impl ActivePanel {
    pub fn next(self) -> Self {
        match self {
            ActivePanel::Files => ActivePanel::Command,
            ActivePanel::Command => ActivePanel::Files,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            ActivePanel::Files => ActivePanel::Command,
            ActivePanel::Command => ActivePanel::Files,
        }
    }
}

pub struct Entry {
    pub name: String,
    pub is_dir: bool,
}

pub struct Model {
    #[allow(dead_code)]
    pub current_dir: PathBuf,
    pub entries: Vec<Entry>,
    pub active_panel: ActivePanel,
    pub selection: usize,
}

impl Model {
    pub fn init() -> io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let entries = read_entries(&current_dir)?;
        Ok(Self {
            current_dir,
            entries,
            active_panel: ActivePanel::Files,
            selection: 0,
        })
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
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
