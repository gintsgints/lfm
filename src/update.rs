use std::path::PathBuf;

use crate::message::Message;
use crate::model::{ActivePanel, Model, TransferOp, TransferProgress};
use crate::ui::{file_panel, pinned_panel};

pub enum Effect {
    None,
    Quit,
    OpenEditor(PathBuf),
    OpenDefault(PathBuf),
    StartCopy(Vec<PathBuf>, PathBuf),
    StartMove(Vec<PathBuf>, PathBuf),
    StartDelete(Vec<PathBuf>),
}

pub fn update(mut model: Model, msg: Message) -> (Model, Effect) {
    match msg {
        Message::Quit => (model, Effect::Quit),
        Message::NextPanel => {
            model.active_panel = model.active_panel.next();
            (model, Effect::None)
        }
        Message::PrevPanel => {
            model.active_panel = model.active_panel.prev();
            (model, Effect::None)
        }
        Message::TogglePinnedPanel => {
            if model.active_panel == ActivePanel::Pinned {
                model.active_panel = model.origin_panel;
            } else {
                model.origin_panel = model.active_panel;
                model.active_panel = ActivePanel::Pinned;
            }
            (model, Effect::None)
        }
        Message::PinCurrentDir => {
            let dir = {
                let origin = origin_file_panel(&model);
                if let Some(entry) = origin.entries.get(origin.selection)
                    && entry.is_dir
                {
                    origin.current_dir.join(&entry.name)
                } else {
                    origin.current_dir.clone()
                }
            };
            if !model.pinned_panel.pins.contains(&dir) {
                model.pinned_panel.pins.push(dir);
            }
            model.active_panel = model.origin_panel;
            (model, Effect::None)
        }
        Message::DeletePinnedDir => {
            let sel = model.pinned_panel.selection;
            if sel < model.pinned_panel.pins.len() {
                model.pinned_panel.pins.remove(sel);
                let count = model.pinned_panel.pins.len();
                if count > 0 {
                    model.pinned_panel.selection = sel.min(count - 1);
                } else {
                    model.pinned_panel.selection = 0;
                }
            }
            (model, Effect::None)
        }
        Message::SelectPinnedDir => {
            if let Some(dir) = model
                .pinned_panel
                .pins
                .get(model.pinned_panel.selection)
                .cloned()
            {
                match model.origin_panel {
                    ActivePanel::LeftFiles => model.left_files.navigate_to(dir),
                    ActivePanel::RightFiles => model.right_files.navigate_to(dir),
                    ActivePanel::Pinned => {}
                }
            }
            model.active_panel = model.origin_panel;
            (model, Effect::None)
        }
        Message::ToggleHelp => {
            model.show_help = !model.show_help;
            (model, Effect::None)
        }
        Message::OpenEditor | Message::OpenDefault => {
            let Some(path) = active_file_path(&model) else {
                return (model, Effect::None);
            };
            let effect = if matches!(msg, Message::OpenEditor) {
                Effect::OpenEditor(path)
            } else {
                Effect::OpenDefault(path)
            };
            (model, effect)
        }
        Message::StartCopy | Message::CancelCopy | Message::ConfirmCopy => update_copy(model, msg),
        Message::StartMove | Message::CancelMove | Message::ConfirmMove => update_move(model, msg),
        Message::DeleteConfirm => update_delete_confirm(model),
        Message::ProgressTick { current, total } => {
            if let Some(p) = &mut model.progress {
                p.current = current;
                p.total = total;
            }
            (model, Effect::None)
        }
        Message::ProgressDone => progress_done(model),
        msg => (dispatch_to_panel(model, msg), Effect::None),
    }
}

fn dispatch_to_panel(mut model: Model, msg: Message) -> Model {
    match model.active_panel {
        ActivePanel::LeftFiles => model.left_files = file_panel::update(model.left_files, msg),
        ActivePanel::RightFiles => model.right_files = file_panel::update(model.right_files, msg),
        ActivePanel::Pinned => model.pinned_panel = pinned_panel::update(model.pinned_panel, msg),
    }
    model
}

fn update_copy(mut model: Model, msg: Message) -> (Model, Effect) {
    match msg {
        Message::StartCopy => {
            let start_dir = model.left_files.current_dir.clone();
            model.right_files.navigate_to(start_dir);
            model.copy_mode = true;
            model.active_panel = ActivePanel::RightFiles;
            (model, Effect::None)
        }
        Message::CancelCopy => {
            model.copy_mode = false;
            model.active_panel = ActivePanel::LeftFiles;
            (model, Effect::None)
        }
        Message::ConfirmCopy => {
            let dst = {
                let rf = &model.right_files;
                match rf.entries.get(rf.selection) {
                    Some(e) if e.is_dir => rf.current_dir.join(&e.name),
                    _ => rf.current_dir.clone(),
                }
            };
            let sources: Vec<PathBuf> = model
                .left_files
                .action_targets()
                .into_iter()
                .map(|t| t.path)
                .collect();
            if sources.is_empty() {
                model.copy_mode = false;
                model.active_panel = ActivePanel::LeftFiles;
                return (model, Effect::None);
            }
            model.copy_mode = false;
            model.active_panel = ActivePanel::LeftFiles;
            model.progress = Some(TransferProgress {
                op: TransferOp::Copy,
                current: 0,
                total: 0,
            });
            (model, Effect::StartCopy(sources, dst))
        }
        _ => (model, Effect::None),
    }
}

fn update_move(mut model: Model, msg: Message) -> (Model, Effect) {
    match msg {
        Message::StartMove => {
            let start_dir = model.left_files.current_dir.clone();
            model.right_files.navigate_to(start_dir);
            model.move_mode = true;
            model.active_panel = ActivePanel::RightFiles;
            (model, Effect::None)
        }
        Message::CancelMove => {
            model.move_mode = false;
            model.active_panel = ActivePanel::LeftFiles;
            (model, Effect::None)
        }
        Message::ConfirmMove => {
            let dst = {
                let rf = &model.right_files;
                match rf.entries.get(rf.selection) {
                    Some(e) if e.is_dir => rf.current_dir.join(&e.name),
                    _ => rf.current_dir.clone(),
                }
            };
            let sources: Vec<PathBuf> = model
                .left_files
                .action_targets()
                .into_iter()
                .map(|t| t.path)
                .collect();
            if sources.is_empty() {
                model.move_mode = false;
                model.active_panel = ActivePanel::LeftFiles;
                return (model, Effect::None);
            }
            model.move_mode = false;
            model.active_panel = ActivePanel::LeftFiles;
            model.progress = Some(TransferProgress {
                op: TransferOp::Move,
                current: 0,
                total: 0,
            });
            (model, Effect::StartMove(sources, dst))
        }
        _ => (model, Effect::None),
    }
}

fn active_file_path(model: &Model) -> Option<std::path::PathBuf> {
    let panel = match model.active_panel {
        ActivePanel::LeftFiles => &model.left_files,
        ActivePanel::RightFiles => &model.right_files,
        ActivePanel::Pinned => return None,
    };
    Some(panel.visible_entries().nth(panel.selection).map_or_else(
        || panel.current_dir.clone(),
        |(_, e)| panel.current_dir.join(&e.name),
    ))
}

fn progress_done(mut model: Model) -> (Model, Effect) {
    model.progress = None;
    model.active_panel = ActivePanel::LeftFiles;
    let left_dir = model.left_files.current_dir.clone();
    model.left_files.navigate_to(left_dir);
    let right_dir = model.right_files.current_dir.clone();
    model.right_files.navigate_to(right_dir);
    (model, Effect::None)
}

fn update_delete_confirm(mut model: Model) -> (Model, Effect) {
    let panel = match model.active_panel {
        ActivePanel::LeftFiles => &mut model.left_files,
        ActivePanel::RightFiles => &mut model.right_files,
        ActivePanel::Pinned => return (model, Effect::None),
    };
    let sources: Vec<PathBuf> = panel
        .delete_targets
        .iter()
        .map(|t| t.path.clone())
        .collect();
    panel.delete_confirm = false;
    panel.delete_targets.clear();
    panel.selected.clear();
    if sources.is_empty() {
        return (model, Effect::None);
    }
    model.progress = Some(TransferProgress {
        op: TransferOp::Delete,
        current: 0,
        total: 0,
    });
    (model, Effect::StartDelete(sources))
}

fn origin_file_panel(model: &Model) -> &file_panel::Model {
    match model.origin_panel {
        ActivePanel::RightFiles => &model.right_files,
        ActivePanel::LeftFiles | ActivePanel::Pinned => &model.left_files,
    }
}
