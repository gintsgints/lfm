use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::model::{ActivePanel, Model};
use crate::ui;

pub fn view(model: &Model, frame: &mut Frame) {
    let area = frame.area();

    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    ui::file_panel::render(
        frame,
        panels[0],
        &model.left_files,
        model.active_panel == ActivePanel::LeftFiles,
    );
    ui::file_panel::render(
        frame,
        panels[1],
        &model.right_files,
        model.active_panel == ActivePanel::RightFiles,
    );

    if model.active_panel == ActivePanel::Pinned {
        ui::pinned_panel::render(frame, area, &model.pinned_panel);
    }
}
