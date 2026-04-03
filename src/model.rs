use std::io;

use crate::ui::file_panel;

#[derive(Clone, Copy, PartialEq)]
pub enum ActivePanel {
    LeftFiles,
    RightFiles,
}

impl ActivePanel {
    pub fn next(self) -> Self {
        match self {
            ActivePanel::LeftFiles => ActivePanel::RightFiles,
            ActivePanel::RightFiles => ActivePanel::LeftFiles,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            ActivePanel::LeftFiles => ActivePanel::RightFiles,
            ActivePanel::RightFiles => ActivePanel::LeftFiles,
        }
    }
}

pub struct Model {
    pub active_panel: ActivePanel,
    pub left_files: file_panel::Model,
    pub right_files: file_panel::Model,
}

impl Model {
    pub fn init() -> io::Result<Self> {
        Ok(Self {
            active_panel: ActivePanel::LeftFiles,
            left_files: file_panel::Model::init()?,
            right_files: file_panel::Model::init()?,
        })
    }
}
