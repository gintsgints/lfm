use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::model::{ActivePanel, Model};
use crate::ui;

pub fn view(model: &Model, frame: &mut Frame) {
    let area = frame.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    ui::file_panel::render(
        frame,
        vertical[0],
        model.entries.iter(),
        model.active_panel == ActivePanel::Files,
        model.selection,
    );
    ui::command_prompt::render(
        frame,
        vertical[1],
        model.active_panel == ActivePanel::Command,
    );
}
