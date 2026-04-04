use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::model::{ActivePanel, Model};
use crate::ui;

pub fn view(model: &Model, frame: &mut Frame) {
    let area = frame.area();

    if model.copy_mode {
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        ui::file_panel::render(
            frame,
            panels[0],
            &model.left_files,
            model.active_panel == ActivePanel::LeftFiles,
            false,
        );
        ui::file_panel::render(
            frame,
            panels[1],
            &model.right_files,
            model.active_panel == ActivePanel::RightFiles,
            true,
        );
    } else {
        ui::file_panel::render(frame, area, &model.left_files, true, false);
    }

    if model.active_panel == ActivePanel::Pinned {
        ui::pinned_panel::render(frame, area, &model.pinned_panel);
    }

    let active_input = match model.active_panel {
        ActivePanel::LeftFiles => Some(&model.left_files.new_path_input),
        ActivePanel::RightFiles => Some(&model.right_files.new_path_input),
        ActivePanel::Pinned => None,
    };
    if let Some(input) = active_input
        && input.active
    {
        ui::input_box::render(frame, area, input, "New path (end with / for directory)");
    }

    let active_file_panel = match model.active_panel {
        ActivePanel::LeftFiles => Some(&model.left_files),
        ActivePanel::RightFiles => Some(&model.right_files),
        ActivePanel::Pinned => None,
    };
    if let Some(fp) = active_file_panel
        && fp.delete_confirm
    {
        let count = fp.delete_targets.len();
        let msg = if count == 1 {
            format!("Delete '{}'?", fp.delete_targets[0].name)
        } else {
            format!("Delete {count} items?")
        };
        ui::confirm_box::render(frame, area, &msg);
    }
}
