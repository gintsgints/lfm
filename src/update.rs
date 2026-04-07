use std::path::PathBuf;

#[cfg(feature = "debug")]
use crate::debug_log;
use crate::message::Message;
use crate::model::{ActivePanel, Model, TransferMode, TransferOp, TransferProgress};
use crate::ui::{file_panel, pinned_panel};

pub enum Effect {
    None,
    Quit,
    OpenEditor(PathBuf),
    OpenDefault(PathBuf),
    StartCopy(Vec<PathBuf>, PathBuf),
    StartMove(Vec<PathBuf>, PathBuf),
    StartCopyRename(PathBuf, PathBuf),
    StartMoveRename(PathBuf, PathBuf),
    StartDelete(Vec<PathBuf>),
}

pub fn update(mut model: Model, msg: Message) -> (Model, Effect) {
    #[cfg(feature = "debug")]
    debug_log!("msg: {msg:?}");
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
        Message::PinCurrentDir => update_pin_current_dir(model),
        Message::DeletePinnedDir => {
            let sel = model.pinned_panel.selection;
            if sel < model.pinned_panel.pins.len() {
                model.pinned_panel.pins.remove(sel);
                let count = model.pinned_panel.pins.len();
                model.pinned_panel.selection = if count > 0 { sel.min(count - 1) } else { 0 };
            }
            (model, Effect::None)
        }
        Message::SelectPinnedDir => update_select_pinned_dir(model),
        Message::ToggleHelp => {
            model.show_help = !model.show_help;
            (model, Effect::None)
        }
        #[cfg(feature = "debug")]
        Message::ToggleDebug => {
            model.show_debug = !model.show_debug;
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
        Message::StartCopy
        | Message::StartCopyRename
        | Message::CancelCopy
        | Message::ConfirmCopy => update_copy(model, msg),
        Message::StartMove
        | Message::StartMoveRename
        | Message::CancelMove
        | Message::ConfirmMove => update_move(model, msg),
        Message::RenameInPlace => {
            let targets = model.left_files.action_targets();
            if targets.is_empty() || targets.len() > 1 {
                return (model, Effect::None);
            }
            open_rename_dialog(&mut model, TransferMode::Rename);
            (model, Effect::None)
        }
        Message::ConfirmRename
        | Message::CancelRename
        | Message::RenameChar(_)
        | Message::RenameBackspace
        | Message::RenameCursorLeft
        | Message::RenameCursorRight => update_rename(model, msg),
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

fn update_pin_current_dir(mut model: Model) -> (Model, Effect) {
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

fn update_select_pinned_dir(mut model: Model) -> (Model, Effect) {
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

fn open_rename_dialog(model: &mut Model, mode: TransferMode) {
    let name = model
        .left_files
        .action_targets()
        .into_iter()
        .next()
        .map(|t| t.name)
        .unwrap_or_default();
    model.rename_input.set_text(name);
    model.rename_input.active = true;
    model.transfer_mode = mode;
}

fn cancel_transfer(model: &mut Model) {
    model.transfer_mode = TransferMode::None;
    model.rename_input.close();
    model.active_panel = ActivePanel::LeftFiles;
}

fn update_copy(mut model: Model, msg: Message) -> (Model, Effect) {
    match msg {
        Message::StartCopy => {
            let start_dir = model.left_files.current_dir.clone();
            model.right_files.navigate_to(start_dir);
            model.transfer_mode = TransferMode::Copy;
            model.active_panel = ActivePanel::RightFiles;
            (model, Effect::None)
        }
        Message::StartCopyRename => {
            let targets = model.left_files.action_targets();
            if targets.is_empty() {
                return (model, Effect::None);
            }
            if targets.len() != 1 {
                // Multi-selection: fall back to regular copy.
                let start_dir = model.left_files.current_dir.clone();
                model.right_files.navigate_to(start_dir);
                model.transfer_mode = TransferMode::Copy;
                model.active_panel = ActivePanel::RightFiles;
                return (model, Effect::None);
            }
            open_rename_dialog(&mut model, TransferMode::CopyRename);
            (model, Effect::None)
        }
        Message::CancelCopy => {
            cancel_transfer(&mut model);
            (model, Effect::None)
        }
        Message::ConfirmCopy => {
            let sources: Vec<PathBuf> = model
                .left_files
                .action_targets()
                .into_iter()
                .map(|t| t.path)
                .collect();
            if sources.is_empty() {
                cancel_transfer(&mut model);
                return (model, Effect::None);
            }
            let dst = dest_dir(&model.right_files);
            if model.transfer_mode.with_rename() {
                let new_name = std::mem::take(&mut model.rename_input.text);
                let src = sources.into_iter().next().unwrap();
                model.transfer_mode = TransferMode::None;
                model.active_panel = ActivePanel::LeftFiles;
                model.progress = Some(TransferProgress {
                    op: TransferOp::Copy,
                    current: 0,
                    total: 0,
                });
                return (model, Effect::StartCopyRename(src, dst.join(new_name)));
            }
            model.transfer_mode = TransferMode::None;
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
            model.transfer_mode = TransferMode::Move;
            model.active_panel = ActivePanel::RightFiles;
            (model, Effect::None)
        }
        Message::StartMoveRename => {
            let targets = model.left_files.action_targets();
            if targets.is_empty() {
                return (model, Effect::None);
            }
            if targets.len() != 1 {
                // Multi-selection: fall back to regular move.
                let start_dir = model.left_files.current_dir.clone();
                model.right_files.navigate_to(start_dir);
                model.transfer_mode = TransferMode::Move;
                model.active_panel = ActivePanel::RightFiles;
                return (model, Effect::None);
            }
            open_rename_dialog(&mut model, TransferMode::MoveRename);
            (model, Effect::None)
        }
        Message::CancelMove => {
            cancel_transfer(&mut model);
            (model, Effect::None)
        }
        Message::ConfirmMove => {
            let sources: Vec<PathBuf> = model
                .left_files
                .action_targets()
                .into_iter()
                .map(|t| t.path)
                .collect();
            if sources.is_empty() {
                cancel_transfer(&mut model);
                return (model, Effect::None);
            }
            let dst = dest_dir(&model.right_files);
            if model.transfer_mode.with_rename() {
                let new_name = std::mem::take(&mut model.rename_input.text);
                let src = sources.into_iter().next().unwrap();
                model.transfer_mode = TransferMode::None;
                model.active_panel = ActivePanel::LeftFiles;
                model.progress = Some(TransferProgress {
                    op: TransferOp::Move,
                    current: 0,
                    total: 0,
                });
                return (model, Effect::StartMoveRename(src, dst.join(new_name)));
            }
            model.transfer_mode = TransferMode::None;
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

fn update_rename(mut model: Model, msg: Message) -> (Model, Effect) {
    match msg {
        Message::RenameChar(c) => {
            model.rename_input.insert(c);
            (model, Effect::None)
        }
        Message::RenameBackspace => {
            model.rename_input.backspace();
            (model, Effect::None)
        }
        Message::RenameCursorLeft => {
            model.rename_input.move_left();
            (model, Effect::None)
        }
        Message::RenameCursorRight => {
            model.rename_input.move_right();
            (model, Effect::None)
        }
        Message::CancelRename => {
            cancel_transfer(&mut model);
            (model, Effect::None)
        }
        Message::ConfirmRename => {
            if model.rename_input.text.is_empty() {
                cancel_transfer(&mut model);
                return (model, Effect::None);
            }
            if model.transfer_mode == TransferMode::Rename {
                // In-place rename: move the file to the same directory under the new name.
                let new_name = std::mem::take(&mut model.rename_input.text);
                model.rename_input.active = false;
                model.transfer_mode = TransferMode::None;
                let Some(target) = model.left_files.action_targets().into_iter().next() else {
                    return (model, Effect::None);
                };
                let dst = target
                    .path
                    .parent()
                    .map_or_else(|| PathBuf::from(&new_name), |p| p.join(&new_name));
                model.progress = Some(TransferProgress {
                    op: TransferOp::Move,
                    current: 0,
                    total: 0,
                });
                return (model, Effect::StartMoveRename(target.path, dst));
            }
            // Deactivate the dialog (keep text) and open the destination panel.
            model.rename_input.active = false;
            let start_dir = model.left_files.current_dir.clone();
            model.right_files.navigate_to(start_dir);
            model.active_panel = ActivePanel::RightFiles;
            (model, Effect::None)
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

fn dest_dir(right: &file_panel::Model) -> PathBuf {
    match right.entries.get(right.selection) {
        Some(e) if e.is_dir => right.current_dir.join(&e.name),
        _ => right.current_dir.clone(),
    }
}

fn origin_file_panel(model: &Model) -> &file_panel::Model {
    match model.origin_panel {
        ActivePanel::RightFiles => &model.right_files,
        ActivePanel::LeftFiles | ActivePanel::Pinned => &model.left_files,
    }
}
