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
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(vertical[0]);

    ui::file_panel::render(
        frame,
        top[0],
        &model.left_files,
        model.active_panel == ActivePanel::LeftFiles,
    );
    ui::file_panel::render(
        frame,
        top[1],
        &model.right_files,
        model.active_panel == ActivePanel::RightFiles,
    );
    ui::command_prompt::render(
        frame,
        vertical[1],
        &model.command_prompt,
        model.active_panel == ActivePanel::Command,
    );
}
