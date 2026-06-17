use std::ffi::OsString;
use std::path::PathBuf;

#[cfg(feature = "debug")]
use crate::debug_log;
use crate::message::Message;
use crate::model::{
    ActivePanel, CommandPicker, ContentSearch, FileFind, FileView, Model, TransferMode, TransferOp,
    TransferProgress,
};
use crate::presets::{self, ExecSpec, OutputMode};
use crate::ui::{capture_popup, file_panel, file_view, help_panel, input_box, pinned_panel};

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
    StartContentSearch { root: PathBuf, query: String },
    StartFileFind { root: PathBuf, query: String },
    RunCommand { spec: RunSpec },
}

pub struct RunSpec {
    pub argv: Option<Vec<OsString>>,
    pub shell: Option<String>,
    pub cwd: PathBuf,
    pub mode: OutputMode,
    pub label: String,
}

#[allow(clippy::too_many_lines)]
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
        Message::DeletePinnedDir => update_delete_pinned_dir(model),
        Message::SelectPinnedDir => update_select_pinned_dir(model),
        Message::ToggleHelp => {
            model.show_help = !model.show_help;
            model.help_selection = 0;
            (model, Effect::None)
        }
        Message::HelpScrollUp => update_help_scroll(model, help_panel::prev_selectable),
        Message::HelpScrollDown => update_help_scroll(model, help_panel::next_selectable),
        Message::SetShiftHeld(held) => {
            model.shift_held = held;
            (model, Effect::None)
        }
        #[cfg(feature = "debug")]
        Message::ToggleDebug => {
            model.show_debug = !model.show_debug;
            (model, Effect::None)
        }
        Message::OpenEditor | Message::OpenDefault => update_open(model, &msg),
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
        Message::ProgressTick { current, total } => update_progress_tick(model, current, total),
        Message::ProgressDone => progress_done(model),
        Message::DismissError => {
            model.error_message = None;
            (model, Effect::None)
        }
        Message::ContentSearch
        | Message::ContentSearchChar(_)
        | Message::ContentSearchBackspace
        | Message::ContentSearchCursorLeft
        | Message::ContentSearchCursorRight
        | Message::ContentSearchToggleFocus
        | Message::ContentSearchCancel
        | Message::ContentSearchUp
        | Message::ContentSearchDown
        | Message::ContentSearchConfirm => update_content_search(model, msg),
        Message::FileFind
        | Message::FileFindChar(_)
        | Message::FileFindBackspace
        | Message::FileFindCursorLeft
        | Message::FileFindCursorRight
        | Message::FileFindToggleFocus
        | Message::FileFindCancel
        | Message::FileFindUp
        | Message::FileFindDown
        | Message::FileFindConfirm => update_file_find(model, msg),
        Message::OpenCommandPicker
        | Message::CommandPickerUp
        | Message::CommandPickerDown
        | Message::CommandPickerCancel
        | Message::CommandPickerConfirm
        | Message::CommandInputChar(_)
        | Message::CommandInputBackspace
        | Message::CommandInputCursorLeft
        | Message::CommandInputCursorRight
        | Message::CommandInputCancel
        | Message::CommandInputConfirm => update_command(model, msg),
        Message::CapturePopupScrollUp
        | Message::CapturePopupScrollDown
        | Message::CapturePopupPageUp
        | Message::CapturePopupPageDown
        | Message::CapturePopupClose => update_capture_popup(model, msg),
        Message::ViewFile => update_view_file(model),
        Message::FileViewScrollUp
        | Message::FileViewScrollDown
        | Message::FileViewPageUp
        | Message::FileViewPageDown
        | Message::FileViewClose => update_file_view(model, msg),
        msg => {
            let (mut m, err) = dispatch_to_panel(model, msg);
            if let Some(e) = err {
                m.error_message = Some(e);
            }
            (m, Effect::None)
        }
    }
}

fn dispatch_to_panel(mut model: Model, msg: Message) -> (Model, Option<String>) {
    match model.active_panel {
        ActivePanel::LeftFiles => {
            let (fp, err) = file_panel::update(model.left_files, msg);
            model.left_files = fp;
            (model, err)
        }
        ActivePanel::RightFiles => {
            let (fp, err) = file_panel::update(model.right_files, msg);
            model.right_files = fp;
            (model, err)
        }
        ActivePanel::Pinned => {
            model.pinned_panel = pinned_panel::update(model.pinned_panel, msg);
            (model, None)
        }
    }
}

fn update_help_scroll(mut model: Model, step: fn(usize) -> usize) -> (Model, Effect) {
    model.help_selection = step(model.help_selection);
    (model, Effect::None)
}

fn update_pin_current_dir(mut model: Model) -> (Model, Effect) {
    let dir = origin_file_panel(&model).current_dir.clone();
    if !model.pinned_panel.pins.contains(&dir) {
        model.pinned_panel.pins.push(dir);
    }
    model.active_panel = model.origin_panel;
    (model, Effect::None)
}

fn update_delete_pinned_dir(mut model: Model) -> (Model, Effect) {
    let sel = model.pinned_panel.selection;
    if sel < model.pinned_panel.pins.len() {
        model.pinned_panel.pins.remove(sel);
        let count = model.pinned_panel.pins.len();
        model.pinned_panel.selection = if count > 0 { sel.min(count - 1) } else { 0 };
    }
    (model, Effect::None)
}

fn update_open(model: Model, msg: &Message) -> (Model, Effect) {
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
                model.pending_select = Some(new_name);
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

fn update_progress_tick(mut model: Model, current: u64, total: u64) -> (Model, Effect) {
    if let Some(p) = &mut model.progress {
        p.current = current;
        p.total = total;
    }
    (model, Effect::None)
}

fn progress_done(mut model: Model) -> (Model, Effect) {
    model.progress = None;
    model.active_panel = ActivePanel::LeftFiles;
    let left_dir = model.left_files.current_dir.clone();
    model.left_files.navigate_to(left_dir);
    let right_dir = model.right_files.current_dir.clone();
    model.right_files.navigate_to(right_dir);
    if let Some(name) = model.pending_select.take() {
        let pos = model
            .left_files
            .visible_entries()
            .position(|(_, e)| e.name == name);
        if let Some(pos) = pos {
            model.left_files.selection = pos;
        }
    }
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

fn update_content_search(mut model: Model, msg: Message) -> (Model, Effect) {
    match msg {
        Message::ContentSearch => {
            if model.active_panel == ActivePanel::Pinned {
                return (model, Effect::None);
            }
            let root = if model.active_panel == ActivePanel::RightFiles {
                model.right_files.current_dir.clone()
            } else {
                model.left_files.current_dir.clone()
            };
            model.content_search = Some(ContentSearch::new(root));
            (model, Effect::None)
        }
        Message::ContentSearchChar(c) => {
            let (query, root) = {
                let Some(cs) = &mut model.content_search else {
                    return (model, Effect::None);
                };
                cs.query.insert(c);
                cs.results.clear();
                cs.selection = 0;
                cs.done = cs.query.text.is_empty();
                (cs.query.text.clone(), cs.root.clone())
            };
            if query.is_empty() {
                (model, Effect::None)
            } else {
                (model, Effect::StartContentSearch { root, query })
            }
        }
        Message::ContentSearchBackspace => {
            let (query, root) = {
                let Some(cs) = &mut model.content_search else {
                    return (model, Effect::None);
                };
                cs.query.backspace();
                cs.results.clear();
                cs.selection = 0;
                cs.done = cs.query.text.is_empty();
                (cs.query.text.clone(), cs.root.clone())
            };
            if query.is_empty() {
                (model, Effect::None)
            } else {
                (model, Effect::StartContentSearch { root, query })
            }
        }
        Message::ContentSearchCursorLeft => {
            if let Some(cs) = &mut model.content_search {
                cs.query.move_left();
            }
            (model, Effect::None)
        }
        Message::ContentSearchCursorRight => {
            if let Some(cs) = &mut model.content_search {
                cs.query.move_right();
            }
            (model, Effect::None)
        }
        Message::ContentSearchToggleFocus => {
            if let Some(cs) = &mut model.content_search {
                cs.input_focused = !cs.input_focused;
            }
            (model, Effect::None)
        }
        Message::ContentSearchCancel => {
            model.content_search = None;
            (model, Effect::None)
        }
        Message::ContentSearchUp => {
            if let Some(cs) = &mut model.content_search {
                if cs.selection == 0 {
                    cs.input_focused = true;
                } else {
                    cs.selection -= 1;
                }
            }
            (model, Effect::None)
        }
        Message::ContentSearchDown => {
            if let Some(cs) = &mut model.content_search
                && !cs.results.is_empty()
            {
                cs.selection = (cs.selection + 1).min(cs.results.len() - 1);
            }
            (model, Effect::None)
        }
        Message::ContentSearchConfirm => confirm_content_search(model),
        _ => (model, Effect::None),
    }
}

fn confirm_content_search(mut model: Model) -> (Model, Effect) {
    let Some(cs) = &model.content_search else {
        return (model, Effect::None);
    };
    let Some(result) = cs.results.get(cs.selection) else {
        return (model, Effect::None);
    };
    let dir = result.path.parent().map(std::path::Path::to_path_buf);
    let name = result
        .path
        .file_name()
        .map(|n: &std::ffi::OsStr| n.to_string_lossy().into_owned());
    model.content_search = None;
    if let Some(dir) = dir {
        model.left_files.navigate_to(dir);
        if let Some(name) = name {
            let pos = model
                .left_files
                .visible_entries()
                .position(|(_, e)| e.name == name);
            if let Some(pos) = pos {
                model.left_files.selection = pos;
            }
        }
    }
    model.active_panel = ActivePanel::LeftFiles;
    (model, Effect::None)
}

fn update_file_find(mut model: Model, msg: Message) -> (Model, Effect) {
    match msg {
        Message::FileFind => {
            if model.active_panel == ActivePanel::Pinned {
                return (model, Effect::None);
            }
            let root = if model.active_panel == ActivePanel::RightFiles {
                model.right_files.current_dir.clone()
            } else {
                model.left_files.current_dir.clone()
            };
            model.file_find = Some(FileFind::new(root));
            (model, Effect::None)
        }
        Message::FileFindChar(c) => {
            let (query, root) = {
                let Some(ff) = &mut model.file_find else {
                    return (model, Effect::None);
                };
                ff.query.insert(c);
                ff.results.clear();
                ff.selection = 0;
                ff.done = ff.query.text.is_empty();
                (ff.query.text.clone(), ff.root.clone())
            };
            if query.is_empty() {
                (model, Effect::None)
            } else {
                (model, Effect::StartFileFind { root, query })
            }
        }
        Message::FileFindBackspace => {
            let (query, root) = {
                let Some(ff) = &mut model.file_find else {
                    return (model, Effect::None);
                };
                ff.query.backspace();
                ff.results.clear();
                ff.selection = 0;
                ff.done = ff.query.text.is_empty();
                (ff.query.text.clone(), ff.root.clone())
            };
            if query.is_empty() {
                (model, Effect::None)
            } else {
                (model, Effect::StartFileFind { root, query })
            }
        }
        Message::FileFindCursorLeft => {
            if let Some(ff) = &mut model.file_find {
                ff.query.move_left();
            }
            (model, Effect::None)
        }
        Message::FileFindCursorRight => {
            if let Some(ff) = &mut model.file_find {
                ff.query.move_right();
            }
            (model, Effect::None)
        }
        Message::FileFindToggleFocus => {
            if let Some(ff) = &mut model.file_find {
                ff.input_focused = !ff.input_focused;
            }
            (model, Effect::None)
        }
        Message::FileFindCancel => {
            model.file_find = None;
            (model, Effect::None)
        }
        Message::FileFindUp => {
            if let Some(ff) = &mut model.file_find {
                if ff.selection == 0 {
                    ff.input_focused = true;
                } else {
                    ff.selection -= 1;
                }
            }
            (model, Effect::None)
        }
        Message::FileFindDown => {
            if let Some(ff) = &mut model.file_find
                && !ff.results.is_empty()
            {
                ff.selection = (ff.selection + 1).min(ff.results.len() - 1);
            }
            (model, Effect::None)
        }
        Message::FileFindConfirm => confirm_file_find(model),
        _ => (model, Effect::None),
    }
}

fn confirm_file_find(mut model: Model) -> (Model, Effect) {
    let Some(ff) = &model.file_find else {
        return (model, Effect::None);
    };
    let Some(result) = ff.results.get(ff.selection) else {
        return (model, Effect::None);
    };
    let dir = result.path.parent().map(std::path::Path::to_path_buf);
    let name = result
        .path
        .file_name()
        .map(|n: &std::ffi::OsStr| n.to_string_lossy().into_owned());
    model.file_find = None;
    if let Some(dir) = dir {
        model.left_files.navigate_to(dir);
        if let Some(name) = name {
            let pos = model
                .left_files
                .visible_entries()
                .position(|(_, e)| e.name == name);
            if let Some(pos) = pos {
                model.left_files.selection = pos;
            }
        }
    }
    model.active_panel = ActivePanel::LeftFiles;
    (model, Effect::None)
}

fn refresh_both_panels(model: &mut Model) {
    let left = model.left_files.current_dir.clone();
    model.left_files.navigate_to(left);
    let right = model.right_files.current_dir.clone();
    model.right_files.navigate_to(right);
}

fn active_panel_dir(model: &Model) -> Option<PathBuf> {
    match model.active_panel {
        ActivePanel::LeftFiles => Some(model.left_files.current_dir.clone()),
        ActivePanel::RightFiles => Some(model.right_files.current_dir.clone()),
        ActivePanel::Pinned => None,
    }
}

fn active_selection(model: &Model) -> Option<(Vec<String>, Vec<PathBuf>)> {
    let panel = match model.active_panel {
        ActivePanel::LeftFiles => &model.left_files,
        ActivePanel::RightFiles => &model.right_files,
        ActivePanel::Pinned => return None,
    };
    let targets = panel.action_targets();
    let names = targets.iter().map(|t| t.name.clone()).collect();
    let paths = targets.iter().map(|t| t.path.clone()).collect();
    Some((names, paths))
}

fn update_command(mut model: Model, msg: Message) -> (Model, Effect) {
    match msg {
        Message::OpenCommandPicker => update_open_command_picker(model),
        Message::CommandPickerUp => update_command_picker_move(model, true),
        Message::CommandPickerDown => update_command_picker_move(model, false),
        Message::CommandPickerCancel => {
            model.command_picker = None;
            (model, Effect::None)
        }
        Message::CommandPickerConfirm => update_command_picker_confirm(model),
        Message::CommandInputChar(c) => {
            if let Some(input) = command_input_mut(&mut model) {
                input.insert(c);
            }
            (model, Effect::None)
        }
        Message::CommandInputBackspace => {
            if let Some(input) = command_input_mut(&mut model) {
                input.backspace();
            }
            (model, Effect::None)
        }
        Message::CommandInputCursorLeft => {
            if let Some(input) = command_input_mut(&mut model) {
                input.move_left();
            }
            (model, Effect::None)
        }
        Message::CommandInputCursorRight => {
            if let Some(input) = command_input_mut(&mut model) {
                input.move_right();
            }
            (model, Effect::None)
        }
        Message::CommandInputCancel => {
            if let Some(cp) = &mut model.command_picker {
                cp.input = None;
            }
            (model, Effect::None)
        }
        Message::CommandInputConfirm => update_command_input_confirm(model),
        _ => (model, Effect::None),
    }
}

fn command_input_mut(model: &mut Model) -> Option<&mut input_box::Model> {
    model
        .command_picker
        .as_mut()
        .and_then(|cp| cp.input.as_mut())
}

fn update_capture_popup(mut model: Model, msg: Message) -> (Model, Effect) {
    let Some(p) = &mut model.capture_popup else {
        return (model, Effect::None);
    };
    let max = capture_popup::line_count(p).saturating_sub(1);
    match msg {
        Message::CapturePopupScrollUp => {
            p.scroll = p.scroll.saturating_sub(1);
        }
        Message::CapturePopupScrollDown => {
            p.scroll = p.scroll.saturating_add(1).min(max);
        }
        Message::CapturePopupPageUp => {
            p.scroll = p.scroll.saturating_sub(10);
        }
        Message::CapturePopupPageDown => {
            p.scroll = p.scroll.saturating_add(10).min(max);
        }
        Message::CapturePopupClose => {
            model.capture_popup = None;
            refresh_both_panels(&mut model);
        }
        _ => {}
    }
    (model, Effect::None)
}

/// Upper bound on the size of a file the viewer will load into memory.
const MAX_VIEW_BYTES: u64 = 5 * 1024 * 1024;

fn update_view_file(mut model: Model) -> (Model, Effect) {
    let target = {
        let panel = match model.active_panel {
            ActivePanel::LeftFiles => &model.left_files,
            ActivePanel::RightFiles => &model.right_files,
            ActivePanel::Pinned => return (model, Effect::None),
        };
        match panel.visible_entries().nth(panel.selection) {
            Some((_, e)) if !e.is_dir => Some((panel.current_dir.join(&e.name), e.name.clone())),
            _ => None,
        }
    };
    let Some((path, name)) = target else {
        return (model, Effect::None);
    };
    match read_text_file(&path) {
        Ok(content) => {
            model.file_view = Some(FileView {
                name,
                content,
                scroll: 0,
            });
        }
        Err(e) => model.error_message = Some(e),
    }
    (model, Effect::None)
}

/// Read `path` as UTF-8 text for the viewer, rejecting oversized or binary files.
fn read_text_file(path: &std::path::Path) -> Result<String, String> {
    let len = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if len > MAX_VIEW_BYTES {
        return Err(format!("file too large to view ({len} bytes)"));
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    detect_text(bytes)
}

/// Interpret bytes as text, treating an embedded NUL or invalid UTF-8 as binary.
pub(crate) fn detect_text(bytes: Vec<u8>) -> Result<String, String> {
    if bytes.contains(&0) {
        return Err("not a text file".to_owned());
    }
    String::from_utf8(bytes).map_err(|_| "not a text file".to_owned())
}

fn update_file_view(mut model: Model, msg: Message) -> (Model, Effect) {
    let Some(v) = &mut model.file_view else {
        return (model, Effect::None);
    };
    let max = file_view::line_count(v).saturating_sub(1);
    match msg {
        Message::FileViewScrollUp => v.scroll = v.scroll.saturating_sub(1),
        Message::FileViewScrollDown => v.scroll = v.scroll.saturating_add(1).min(max),
        Message::FileViewPageUp => v.scroll = v.scroll.saturating_sub(10),
        Message::FileViewPageDown => v.scroll = v.scroll.saturating_add(10).min(max),
        Message::FileViewClose => model.file_view = None,
        _ => {}
    }
    (model, Effect::None)
}

fn update_open_command_picker(mut model: Model) -> (Model, Effect) {
    if model.active_panel == ActivePanel::Pinned {
        return (model, Effect::None);
    }
    match presets::load_or_create() {
        Ok(presets) => {
            model.command_picker = Some(CommandPicker::new(presets));
        }
        Err(e) => {
            model.error_message = Some(format!("commands.json: {e}"));
        }
    }
    (model, Effect::None)
}

fn update_command_picker_move(mut model: Model, up: bool) -> (Model, Effect) {
    if let Some(cp) = &mut model.command_picker
        && cp.input.is_none()
        && !cp.presets.is_empty()
    {
        if up {
            cp.selection = cp.selection.saturating_sub(1);
        } else {
            cp.selection = (cp.selection + 1).min(cp.presets.len() - 1);
        }
    }
    (model, Effect::None)
}

fn update_command_picker_confirm(mut model: Model) -> (Model, Effect) {
    let needs_input = {
        let Some(cp) = &model.command_picker else {
            return (model, Effect::None);
        };
        let Some(preset) = cp.presets.get(cp.selection) else {
            return (model, Effect::None);
        };
        preset.needs_input()
    };
    if needs_input {
        if let Some(cp) = &mut model.command_picker {
            let mut input = input_box::Model::new();
            input.open();
            cp.input = Some(input);
        }
        return (model, Effect::None);
    }
    run_selected_preset(model, "")
}

fn update_command_input_confirm(model: Model) -> (Model, Effect) {
    let input_text = model
        .command_picker
        .as_ref()
        .and_then(|cp| cp.input.as_ref())
        .map(|i| i.text.clone())
        .unwrap_or_default();
    run_selected_preset(model, &input_text)
}

fn run_selected_preset(mut model: Model, input: &str) -> (Model, Effect) {
    let (preset, cwd) = {
        let Some(cp) = &model.command_picker else {
            return (model, Effect::None);
        };
        let Some(preset) = cp.presets.get(cp.selection).cloned() else {
            return (model, Effect::None);
        };
        let Some(cwd) = active_panel_dir(&model) else {
            model.command_picker = None;
            return (model, Effect::None);
        };
        (preset, cwd)
    };

    let (names, paths) = active_selection(&model).unwrap_or_default();

    if preset.references_files() && names.is_empty() {
        model.command_picker = None;
        model.error_message = Some(format!(
            "'{}' requires a selected file (none available)",
            preset.label
        ));
        return (model, Effect::None);
    }

    let spec = match preset.expand(&names, &paths, &cwd, input) {
        Ok(s) => s,
        Err(e) => {
            model.command_picker = None;
            model.error_message = Some(format!("'{}': {e}", preset.label));
            return (model, Effect::None);
        }
    };

    let (argv, shell) = match spec {
        ExecSpec::Argv(v) => (Some(v), None),
        ExecSpec::Shell(s) => (None, Some(s)),
    };

    let run = RunSpec {
        argv,
        shell,
        cwd,
        mode: preset.output,
        label: preset.label.clone(),
    };
    model.command_picker = None;
    (model, Effect::RunCommand { spec: run })
}
