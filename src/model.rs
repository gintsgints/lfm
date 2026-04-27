use std::{io, path::PathBuf};

use crate::search::SearchResult;
use crate::state::PersistedState;
use crate::ui::{file_panel, input_box, pinned_panel};

pub struct ContentSearch {
    pub root: PathBuf,
    pub query: input_box::Model,
    pub results: Vec<SearchResult>,
    pub selection: usize,
    pub done: bool,
    pub input_focused: bool,
}

impl ContentSearch {
    pub fn new(root: PathBuf) -> Self {
        let mut query = input_box::Model::new();
        query.open();
        Self {
            root,
            query,
            results: Vec::new(),
            selection: 0,
            done: true,
            input_focused: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum TransferOp {
    Copy,
    Move,
    Delete,
}

pub struct TransferProgress {
    pub op: TransferOp,
    pub current: u64,
    pub total: u64,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum TransferMode {
    #[default]
    None,
    Copy,
    CopyRename,
    Move,
    MoveRename,
    Rename,
}

impl TransferMode {
    pub fn is_copy(self) -> bool {
        matches!(self, Self::Copy | Self::CopyRename)
    }

    pub fn is_move(self) -> bool {
        matches!(self, Self::Move | Self::MoveRename)
    }

    pub fn with_rename(self) -> bool {
        matches!(self, Self::CopyRename | Self::MoveRename)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ActivePanel {
    LeftFiles,
    RightFiles,
    Pinned,
}

impl ActivePanel {
    pub fn next(self) -> Self {
        match self {
            ActivePanel::LeftFiles => ActivePanel::RightFiles,
            ActivePanel::RightFiles => ActivePanel::LeftFiles,
            ActivePanel::Pinned => ActivePanel::Pinned,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            ActivePanel::LeftFiles => ActivePanel::RightFiles,
            ActivePanel::RightFiles => ActivePanel::LeftFiles,
            ActivePanel::Pinned => ActivePanel::Pinned,
        }
    }
}

pub struct Model {
    pub active_panel: ActivePanel,
    /// The file panel that was active when the pinned panel was opened.
    pub origin_panel: ActivePanel,
    pub left_files: file_panel::Model,
    pub right_files: file_panel::Model,
    pub pinned_panel: pinned_panel::Model,
    pub transfer_mode: TransferMode,
    pub rename_input: input_box::Model,
    pub show_help: bool,
    #[cfg(feature = "debug")]
    pub show_debug: bool,
    pub progress: Option<TransferProgress>,
    /// Name to select in the left panel after the next `progress_done`.
    pub pending_select: Option<String>,
    pub error_message: Option<String>,
    pub content_search: Option<ContentSearch>,
}

impl Model {
    pub fn init(persisted: PersistedState) -> io::Result<Self> {
        let cwd = std::env::current_dir()?;
        Ok(Self {
            active_panel: ActivePanel::LeftFiles,
            origin_panel: ActivePanel::LeftFiles,
            left_files: file_panel::Model::init(cwd.clone())?,
            right_files: file_panel::Model::init(cwd)?,
            pinned_panel: pinned_panel::Model::with_pins(persisted.pins),
            transfer_mode: TransferMode::None,
            rename_input: input_box::Model::new(),
            show_help: false,
            #[cfg(feature = "debug")]
            show_debug: true,
            progress: None,
            pending_select: None,
            error_message: None,
            content_search: None,
        })
    }

    pub fn to_persisted(&self) -> PersistedState {
        PersistedState {
            pins: self.pinned_panel.pins.clone(),
        }
    }
}
