use std::io;

use crate::ui::{command_prompt, file_panel};

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

pub struct Model {
    pub active_panel: ActivePanel,
    pub file_panel: file_panel::Model,
    pub command_prompt: command_prompt::Model,
}

impl Model {
    pub fn init() -> io::Result<Self> {
        Ok(Self {
            active_panel: ActivePanel::Files,
            file_panel: file_panel::Model::init()?,
            command_prompt: command_prompt::Model {},
        })
    }
}
