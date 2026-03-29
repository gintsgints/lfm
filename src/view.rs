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

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Min(0)])
        .split(vertical[0]);

    ui::dir_panel::render(
        frame,
        top[0],
        model.dirs(),
        model.active_panel == ActivePanel::Dirs,
        model.dir_selection,
    );
    ui::file_panel::render(
        frame,
        top[1],
        model.files(),
        model.active_panel == ActivePanel::Files,
        model.file_selection,
    );
    ui::command_prompt::render(
        frame,
        vertical[1],
        model.active_panel == ActivePanel::Command,
    );
}
