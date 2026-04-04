use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::model::{ActivePanel, Model};
use crate::theme;
use crate::ui;

pub fn view(model: &Model, frame: &mut Frame) {
    let area = frame.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let main_area = vertical[0];
    let hint_area = vertical[1];

    if model.copy_mode {
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(main_area);

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
        ui::file_panel::render(frame, main_area, &model.left_files, true, false);
    }

    let hint = hint_line(model);
    frame.render_widget(Paragraph::new(hint), hint_area);

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

    if model.show_help {
        ui::help_panel::render(frame, area);
    }

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

fn key(label: &'static str) -> Span<'static> {
    Span::styled(label, Style::default().fg(theme::ACTIVE_BORDER))
}

fn desc(label: &'static str) -> Span<'static> {
    Span::styled(label, Style::default().fg(theme::INACTIVE_BORDER))
}

fn hint_line(model: &Model) -> Line<'static> {
    let active_panel = match model.active_panel {
        ActivePanel::LeftFiles => Some(&model.left_files),
        ActivePanel::RightFiles => Some(&model.right_files),
        ActivePanel::Pinned => None,
    };
    let in_new_path = active_panel.is_some_and(|p| p.new_path_input.active);
    let in_delete = active_panel.is_some_and(|p| p.delete_confirm);
    let in_filter = active_panel.is_some_and(|p| p.search.active);

    if model.show_help {
        Line::from(vec![
            key(" Esc"),
            desc(" / "),
            key("?"),
            desc(" close help"),
        ])
    } else if in_delete {
        Line::from(vec![
            key(" Enter"),
            desc(" confirm delete  "),
            key("Esc"),
            desc(" cancel"),
        ])
    } else if in_new_path {
        Line::from(vec![
            key(" Enter"),
            desc(" create  "),
            key("Esc"),
            desc(" cancel"),
        ])
    } else if in_filter {
        Line::from(vec![
            key(" Enter"),
            desc(" / "),
            key("Esc"),
            desc(" exit filter"),
        ])
    } else if model.copy_mode {
        Line::from(vec![
            key(" Enter"),
            desc(" copy here  "),
            key("Tab"),
            desc(" switch panel  "),
            key("Esc"),
            desc(" cancel"),
        ])
    } else if model.active_panel == ActivePanel::Pinned {
        Line::from(vec![
            key(" p"),
            desc(" pin  "),
            key("Enter"),
            desc("/"),
            key("Space"),
            desc(" go  "),
            key("d"),
            desc(" delete  "),
            key("Esc"),
            desc(" close"),
        ])
    } else {
        Line::from(vec![
            key(" q"),
            desc(" quit  "),
            key("?"),
            desc(" help  "),
            key("/"),
            desc(" filter  "),
            key("c"),
            desc(" copy  "),
            key("d"),
            desc(" delete  "),
            key("n"),
            desc(" new  "),
            key("p"),
            desc(" pins  "),
            key("e"),
            desc(" editor"),
        ])
    }
}
