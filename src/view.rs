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

    #[cfg(feature = "debug")]
    let vertical = if model.show_debug {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(8),
                Constraint::Length(1),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area)
    };
    #[cfg(not(feature = "debug"))]
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let main_area = vertical[0];
    let hint_area = vertical[vertical.len() - 1];

    if (model.transfer_mode.is_copy() || model.transfer_mode.is_move())
        && !model.rename_input.active
    {
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
            false,
        );
        ui::file_panel::render(
            frame,
            panels[1],
            &model.right_files,
            model.active_panel == ActivePanel::RightFiles,
            model.transfer_mode.is_copy(),
            model.transfer_mode.is_move(),
        );
    } else {
        ui::file_panel::render(frame, main_area, &model.left_files, true, false, false);
    }

    let hint = hint_line(model);
    frame.render_widget(Paragraph::new(hint), hint_area);

    #[cfg(feature = "debug")]
    if model.show_debug {
        render_debug_panel(frame, vertical[1]);
    }

    render_overlays(model, frame, area);
}

fn render_overlays(model: &Model, frame: &mut Frame, area: ratatui::layout::Rect) {
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

    let active_goto = match model.active_panel {
        ActivePanel::LeftFiles => Some(&model.left_files.goto_input),
        ActivePanel::RightFiles => Some(&model.right_files.goto_input),
        ActivePanel::Pinned => None,
    };
    if let Some(input) = active_goto
        && input.active
    {
        ui::input_box::render(frame, area, input, "Go to path");
    }

    let active_file_panel = match model.active_panel {
        ActivePanel::LeftFiles => Some(&model.left_files),
        ActivePanel::RightFiles => Some(&model.right_files),
        ActivePanel::Pinned => None,
    };

    if model.rename_input.active {
        let title = if model.transfer_mode.is_copy() {
            "Copy with rename"
        } else if model.transfer_mode.is_move() {
            "Move with rename"
        } else {
            "Rename"
        };
        ui::input_box::render(frame, area, &model.rename_input, title);
    }

    if model.show_help {
        ui::help_panel::render(frame, area, model.help_selection);
    }

    if let Some(progress) = &model.progress {
        ui::progress_bar::render(frame, area, progress);
    }

    if let Some(cs) = &model.content_search {
        ui::content_search_panel::render(frame, area, cs);
    }

    if let Some(ff) = &model.file_find {
        ui::file_find_panel::render(frame, area, ff);
    }

    if let Some(cp) = &model.command_picker {
        ui::command_picker::render(frame, area, cp);
    }

    if let Some(pop) = &model.capture_popup {
        ui::capture_popup::render(frame, area, pop);
    }

    if let Some(fv) = &model.file_view {
        ui::file_view::render(frame, area, fv);
    }

    if let Some(pending) = &model.pending_overwrite {
        ui::confirm_box::render(frame, area, &overwrite_message(&pending.conflicts));
    }

    if let Some(msg) = &model.error_message {
        ui::error_box::render(frame, area, msg);
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

fn overwrite_message(conflicts: &[String]) -> String {
    match conflicts {
        [name] => format!("Overwrite '{name}'?"),
        _ => format!("Overwrite {} existing items?", conflicts.len()),
    }
}

fn key(label: &'static str) -> Span<'static> {
    Span::styled(label, Style::default().fg(theme::active_border()))
}

fn desc(label: &'static str) -> Span<'static> {
    Span::styled(label, Style::default().fg(theme::inactive_border()))
}

fn active_panel_hints(model: &Model) -> Option<Line<'static>> {
    let active_panel = match model.active_panel {
        ActivePanel::LeftFiles => Some(&model.left_files),
        ActivePanel::RightFiles => Some(&model.right_files),
        ActivePanel::Pinned => None,
    }?;
    let in_new_path = active_panel.new_path_input.active;
    let in_goto = active_panel.goto_input.active;
    let in_delete = active_panel.delete_confirm;
    let in_filter = active_panel.search.active;
    let filter_locked = !active_panel.search.active && !active_panel.search.text.is_empty();

    if in_goto {
        Some(Line::from(vec![
            key(" Enter"),
            desc(" go  "),
            key("Esc"),
            desc(" cancel"),
        ]))
    } else if in_delete {
        Some(Line::from(vec![
            key(" Enter"),
            desc(" confirm delete  "),
            key("Esc"),
            desc(" cancel"),
        ]))
    } else if in_new_path {
        Some(Line::from(vec![
            key(" Enter"),
            desc(" create  "),
            key("Esc"),
            desc(" cancel"),
        ]))
    } else if in_filter {
        let mut spans = vec![
            key(" Enter"),
            desc(" / "),
            key("Tab"),
            desc(" / "),
            key("↓"),
            desc(" to panel  "),
            key("Esc"),
            desc(" clear filter  "),
        ];
        spans.extend(hint_spans(model.shift_held));
        Some(Line::from(spans))
    } else if filter_locked {
        let mut spans = vec![
            key(" /"),
            desc(" / "),
            key("Tab"),
            desc(" edit filter  "),
            key("Esc"),
            desc(" clear filter  "),
        ];
        spans.extend(hint_spans(model.shift_held));
        Some(Line::from(spans))
    } else {
        None
    }
}

fn file_view_hint() -> Line<'static> {
    Line::from(vec![
        key(" ↑/↓ j/k"),
        desc(" scroll  "),
        key("PgUp/PgDn"),
        desc(" page  "),
        key("Esc"),
        desc(" / "),
        key("q"),
        desc(" close"),
    ])
}

fn pinned_panel_hint() -> Line<'static> {
    Line::from(vec![
        key(" ↑/↓ j/k"),
        desc(" move  "),
        key("p"),
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
}

fn hint_line(model: &Model) -> Line<'static> {
    if model.file_view.is_some() {
        return file_view_hint();
    }
    if model.capture_popup.is_some() {
        return Line::from(vec![
            key(" j/k"),
            desc(" scroll  "),
            key("PgUp/PgDn"),
            desc(" page  "),
            key("Esc"),
            desc(" / "),
            key("Enter"),
            desc(" close"),
        ]);
    }
    if let Some(cp) = &model.command_picker {
        return if cp.input.is_some() {
            Line::from(vec![
                key(" Enter"),
                desc(" run  "),
                key("Esc"),
                desc(" back"),
            ])
        } else {
            Line::from(vec![
                key(" j/k"),
                desc(" move  "),
                key("Enter"),
                desc(" select  "),
                key("Esc"),
                desc(" close"),
            ])
        };
    }
    if model.pending_overwrite.is_some() {
        Line::from(vec![
            key(" Enter"),
            desc(" overwrite  "),
            key("Esc"),
            desc(" cancel"),
        ])
    } else if let Some(cs) = &model.content_search {
        content_search_hint(cs.input_focused)
    } else if let Some(ff) = &model.file_find {
        content_search_hint(ff.input_focused)
    } else if model.error_message.is_some() {
        Line::from(vec![
            key(" Enter"),
            desc(" / "),
            key("Esc"),
            desc(" dismiss error"),
        ])
    } else if model.rename_input.active {
        Line::from(vec![
            key(" Enter"),
            desc(" confirm  "),
            key("Esc"),
            desc(" cancel rename"),
        ])
    } else if model.show_help {
        Line::from(vec![
            key(" j/k"),
            desc(" scroll  "),
            key("Esc"),
            desc(" / "),
            key("?"),
            desc(" / "),
            key("q"),
            desc(" close help"),
        ])
    } else if let Some(line) = active_panel_hints(model) {
        line
    } else if model.transfer_mode.is_copy() {
        Line::from(vec![
            key(" Enter"),
            desc(" copy here  "),
            key("Tab"),
            desc(" switch panel  "),
            key("Esc"),
            desc(" cancel"),
        ])
    } else if model.transfer_mode.is_move() {
        Line::from(vec![
            key(" Enter"),
            desc(" move here  "),
            key("Tab"),
            desc(" switch panel  "),
            key("Esc"),
            desc(" cancel"),
        ])
    } else if model.active_panel == ActivePanel::Pinned {
        pinned_panel_hint()
    } else {
        normal_hint_line(model.shift_held)
    }
}

#[cfg(feature = "debug")]
fn render_debug_panel(frame: &mut Frame, area: ratatui::layout::Rect) {
    use ratatui::layout::Alignment;
    use ratatui::widgets::{Block, Borders, List, ListItem};
    let messages = crate::debug::snapshot();
    // The list renders top-down, so show only the newest messages that fit
    // (minus 2 rows for the top/bottom borders) to keep the latest visible.
    let visible = area.height.saturating_sub(2) as usize;
    let start = messages.len().saturating_sub(visible);
    let items: Vec<ListItem> = messages[start..]
        .iter()
        .map(|m| ListItem::new(m.as_str()))
        .collect();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::inactive_border()))
                .title_alignment(Alignment::Left)
                .title(" debug "),
        )
        .highlight_style(Style::default());
    frame.render_widget(list, area);
}

fn content_search_hint(input_focused: bool) -> Line<'static> {
    if input_focused {
        Line::from(vec![
            key(" Tab"),
            desc(" results  "),
            key("Esc"),
            desc(" cancel search"),
        ])
    } else {
        Line::from(vec![
            key(" Enter"),
            desc(" select  "),
            key("Tab"),
            desc(" input  "),
            key("Esc"),
            desc(" cancel search"),
        ])
    }
}

fn normal_hint_spans() -> Vec<Span<'static>> {
    vec![
        key(" q"),
        desc(" quit  "),
        key("?"),
        desc(" help  "),
        key("/"),
        desc(" filter  "),
        key("r"),
        desc(" rename  "),
        key("c"),
        desc(" copy  "),
        key("m"),
        desc(" move  "),
        key("d"),
        desc(" delete  "),
        key("n"),
        desc(" new  "),
        key("g"),
        desc(" goto  "),
        key("p"),
        desc(" pins  "),
        key("v"),
        desc(" view  "),
        key("e"),
        desc(" editor  "),
        key("x"),
        desc(" run  "),
        key("s"),
        desc(" sort"),
    ]
}

/// The shifted command set, shown while Shift is held (see [`hint_spans`]).
fn shifted_hint_spans() -> Vec<Span<'static>> {
    vec![
        key(" J"),
        desc(" mark ↓  "),
        key("K"),
        desc(" mark ↑  "),
        key("C"),
        desc(" copy+rename  "),
        key("M"),
        desc(" move+rename  "),
        key("S"),
        desc(" search  "),
        key("F"),
        desc(" find"),
    ]
}

/// Pick the normal or shifted hint set depending on whether Shift is held.
fn hint_spans(shift_held: bool) -> Vec<Span<'static>> {
    if shift_held {
        shifted_hint_spans()
    } else {
        normal_hint_spans()
    }
}

fn normal_hint_line(shift_held: bool) -> Line<'static> {
    Line::from(hint_spans(shift_held))
}
