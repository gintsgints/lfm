use std::io;

use crate::ui::{file_panel, pinned_panel};

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
}

impl Model {
    pub fn init() -> io::Result<Self> {
        Ok(Self {
            active_panel: ActivePanel::LeftFiles,
            origin_panel: ActivePanel::LeftFiles,
            left_files: file_panel::Model::init()?,
            right_files: file_panel::Model::init()?,
            pinned_panel: pinned_panel::Model::new(),
        })
    }
}
