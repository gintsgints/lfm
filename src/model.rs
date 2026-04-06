use std::io;

use crate::state::PersistedState;
use crate::ui::{file_panel, pinned_panel};

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
    pub copy_mode: bool,
    pub move_mode: bool,
    pub show_help: bool,
    pub progress: Option<TransferProgress>,
}

impl Model {
    pub fn init(persisted: PersistedState) -> io::Result<Self> {
        let mut left_files = file_panel::Model::init()?;
        if let Some(dir) = persisted.left_dir {
            left_files.navigate_to(dir);
        }
        Ok(Self {
            active_panel: ActivePanel::LeftFiles,
            origin_panel: ActivePanel::LeftFiles,
            left_files,
            right_files: file_panel::Model::init()?,
            pinned_panel: pinned_panel::Model::with_pins(persisted.pins),
            copy_mode: false,
            move_mode: false,
            show_help: false,
            progress: None,
        })
    }

    pub fn to_persisted(&self) -> PersistedState {
        PersistedState {
            left_dir: Some(self.left_files.current_dir.clone()),
            pins: self.pinned_panel.pins.clone(),
        }
    }
}
