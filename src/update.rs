use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tui_view::{FormatView, ViewState, plugins::plaintext::PlainTextView, plugins::zip::ZipView};

#[cfg(feature = "debug")]
use crate::debug_log;
use crate::image_view;
use crate::message::{EditOp, Field, Message, NavOp, SearchKind, Surface};
use crate::model::{
    ActivePanel, CommandPicker, FileView, InputField, Located, Model, PendingKind,
    PendingOverwrite, ResultPanel, TransferMode, TransferOp, TransferProgress, ViewContent,
};
use crate::presets::{self, ExecSpec, OutputMode};
use crate::ui::{capture_view, file_panel, help_panel, input_box, pinned_panel};

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
    /// Make sure the content-search index for `root` exists (and start building
    /// it if not) without running a query yet.
    PrepareContentSearch {
        root: PathBuf,
    },
    StartContentSearch {
        root: PathBuf,
        query: String,
        /// Glob patterns limiting which files are grepped; empty means all.
        mask: String,
    },
    /// Make sure the file-find index for `root` exists (and start building it if
    /// not) without running a query yet.
    PrepareFileFind {
        root: PathBuf,
    },
    StartFileFind {
        root: PathBuf,
        query: String,
        /// Glob patterns limiting which names are ranked; empty means all.
        mask: String,
    },
    RunCommand {
        spec: RunSpec,
    },
}

pub struct RunSpec {
    pub argv: Option<Vec<OsString>>,
    pub shell: Option<String>,
    pub cwd: PathBuf,
    pub mode: OutputMode,
    pub label: String,
}

pub fn update(model: Model, msg: Message) -> (Model, Effect) {
    let (mut model, effect) = update_message(model, msg);
    // Whatever the message did, the viewer panel follows the file list: if the
    // highlighted file changed, reload the viewer for it.
    sync_file_view(&mut model);
    (model, effect)
}

#[allow(clippy::too_many_lines)]
fn update_message(mut model: Model, msg: Message) -> (Model, Effect) {
    #[cfg(feature = "debug")]
    debug_log!("msg: {msg:?}");
    match msg {
        Message::Quit => (model, Effect::Quit),
        Message::NextPanel | Message::PrevPanel => {
            // While the viewer panel occupies the right half, Tab moves between
            // the file list and the viewer instead of between the two lists.
            if model.file_view.is_some() && model.transfer_mode == TransferMode::None {
                model.file_view_focused = !model.file_view_focused;
            } else if matches!(msg, Message::NextPanel) {
                model.active_panel = model.active_panel.next();
            } else {
                model.active_panel = model.active_panel.prev();
            }
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
        Message::Nav(Surface::Help, op) => update_help_scroll(model, op),
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
        | Message::Cancel(Field::Rename)
        | Message::Edit(Field::Rename, _) => update_rename(model, msg),
        Message::DeleteConfirm => update_delete_confirm(model),
        Message::ProgressTick { current, total } => update_progress_tick(model, current, total),
        Message::ProgressDone => progress_done(model),
        Message::OverwriteConfirm => overwrite_confirm(model),
        Message::OverwriteCancel => {
            model.pending_overwrite = None;
            model.pending_select = None;
            model.active_panel = ActivePanel::LeftFiles;
            (model, Effect::None)
        }
        Message::DismissError => {
            model.error_message = None;
            (model, Effect::None)
        }
        Message::SearchOpen(kind)
        | Message::SearchToggleFocus(kind)
        | Message::Close(Surface::Search(kind))
        | Message::SearchConfirm(kind)
        | Message::Edit(Field::SearchQuery(kind), _)
        | Message::Nav(Surface::Search(kind), _) => update_search(model, kind, msg),
        Message::OpenCommandPicker
        | Message::Nav(Surface::CommandPicker, _)
        | Message::Close(Surface::CommandPicker)
        | Message::CommandPickerConfirm
        | Message::CommandPickerShortcut(_)
        | Message::Edit(Field::CommandInput, _)
        | Message::Cancel(Field::CommandInput)
        | Message::CommandInputConfirm => update_command(model, msg),
        Message::Close(Surface::Capture) | Message::Nav(Surface::Capture, _) => {
            update_capture_view(model, msg)
        }
        Message::ViewFile => update_view_file(model),
        Message::Close(Surface::FileView) | Message::Nav(Surface::FileView, _) => {
            update_file_view(model, msg)
        }
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

fn update_help_scroll(mut model: Model, op: NavOp) -> (Model, Effect) {
    match op {
        NavOp::Up => model.help_selection = help_panel::prev_selectable(model.help_selection),
        NavOp::Down => model.help_selection = help_panel::next_selectable(model.help_selection),
        // The help list neither pages nor marks.
        _ => {}
    }
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

/// Open the right panel as the destination for `mode`. It lists directories
/// only: a copy or move target is always a directory.
fn open_target_panel(model: &mut Model, mode: TransferMode) {
    let start_dir = model.left_files.current_dir.clone();
    model.right_files.navigate_to(start_dir);
    model.right_files.set_dirs_only(true);
    model.transfer_mode = mode;
    model.active_panel = ActivePanel::RightFiles;
}

fn cancel_transfer(model: &mut Model) {
    model.transfer_mode = TransferMode::None;
    model.rename_input.close();
    model.right_files.set_dirs_only(false);
    model.active_panel = ActivePanel::LeftFiles;
}

fn pending_effect(kind: PendingKind) -> Effect {
    match kind {
        PendingKind::Copy(sources, dst) => Effect::StartCopy(sources, dst),
        PendingKind::Move(sources, dst) => Effect::StartMove(sources, dst),
        PendingKind::CopyRename(src, dst) => Effect::StartCopyRename(src, dst),
        PendingKind::MoveRename(src, dst) => Effect::StartMoveRename(src, dst),
    }
}

/// What to tell the user about a transfer that would land its sources exactly
/// where they already are, or `None` when there is real work to do. Reported
/// before any state is torn down, so the picker or the rename dialog it came
/// from is still open once the message is dismissed.
fn same_location_message(kind: &PendingKind) -> Option<String> {
    if !kind.is_same_location() {
        return None;
    }
    Some(
        match kind {
            PendingKind::Copy(..) => "Copy target is the source folder — pick another folder.",
            PendingKind::Move(..) => "Move target is the source folder — pick another folder.",
            // Reached either from the rename dialog (in-place rename to the
            // same name) or from the destination picker, so name both ways out.
            PendingKind::CopyRename(..) | PendingKind::MoveRename(..) => {
                "Target is the source itself — pick another folder or name."
            }
        }
        .to_owned(),
    )
}

/// Build the transfer for the destination now shown in the right panel and
/// launch it. A destination that is the folder the sources already live in
/// leaves the picker exactly as it is, so the user can dismiss the message and
/// walk to another folder.
fn confirm_transfer(model: &mut Model, op: TransferOp) -> Effect {
    let sources: Vec<PathBuf> = model
        .left_files
        .action_targets()
        .into_iter()
        .map(|t| t.path)
        .collect();
    if sources.is_empty() {
        cancel_transfer(model);
        return Effect::None;
    }
    let dst = model.right_files.current_dir.clone();
    let moving = op == TransferOp::Move;
    let kind = if model.transfer_mode.with_rename() {
        let target = dst.join(&model.rename_input.text);
        let src = sources.into_iter().next().unwrap();
        if moving {
            PendingKind::MoveRename(src, target)
        } else {
            PendingKind::CopyRename(src, target)
        }
    } else if moving {
        PendingKind::Move(sources, dst)
    } else {
        PendingKind::Copy(sources, dst)
    };
    if let Some(message) = same_location_message(&kind) {
        model.error_message = Some(message);
        return Effect::None;
    }
    model.transfer_mode = TransferMode::None;
    model.rename_input.close();
    model.right_files.set_dirs_only(false);
    model.active_panel = ActivePanel::LeftFiles;
    begin_transfer(model, kind)
}

/// Launch a confirmed copy/move, or, if it would overwrite existing entries,
/// hold it back behind an overwrite prompt instead.
fn begin_transfer(model: &mut Model, kind: PendingKind) -> Effect {
    let conflicts = kind.conflicts();
    if conflicts.is_empty() {
        model.progress = Some(TransferProgress {
            op: kind.op(),
            current: 0,
            total: 0,
        });
        pending_effect(kind)
    } else {
        model.pending_overwrite = Some(PendingOverwrite { kind, conflicts });
        Effect::None
    }
}

fn overwrite_confirm(mut model: Model) -> (Model, Effect) {
    let Some(pending) = model.pending_overwrite.take() else {
        return (model, Effect::None);
    };
    model.progress = Some(TransferProgress {
        op: pending.kind.op(),
        current: 0,
        total: 0,
    });
    (model, pending_effect(pending.kind))
}

fn update_copy(mut model: Model, msg: Message) -> (Model, Effect) {
    match msg {
        Message::StartCopy => {
            open_target_panel(&mut model, TransferMode::Copy);
            (model, Effect::None)
        }
        Message::StartCopyRename => {
            let targets = model.left_files.action_targets();
            if targets.is_empty() {
                return (model, Effect::None);
            }
            if targets.len() != 1 {
                // Multi-selection: fall back to regular copy.
                open_target_panel(&mut model, TransferMode::Copy);
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
            let effect = confirm_transfer(&mut model, TransferOp::Copy);
            (model, effect)
        }
        _ => (model, Effect::None),
    }
}

fn update_move(mut model: Model, msg: Message) -> (Model, Effect) {
    match msg {
        Message::StartMove => {
            open_target_panel(&mut model, TransferMode::Move);
            (model, Effect::None)
        }
        Message::StartMoveRename => {
            let targets = model.left_files.action_targets();
            if targets.is_empty() {
                return (model, Effect::None);
            }
            if targets.len() != 1 {
                // Multi-selection: fall back to regular move.
                open_target_panel(&mut model, TransferMode::Move);
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
            let effect = confirm_transfer(&mut model, TransferOp::Move);
            (model, effect)
        }
        _ => (model, Effect::None),
    }
}

fn update_rename(mut model: Model, msg: Message) -> (Model, Effect) {
    match msg {
        Message::Edit(_, op) => {
            input_box::apply(&mut model.rename_input, op);
            (model, Effect::None)
        }
        Message::Cancel(_) => {
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
                let Some(target) = model.left_files.action_targets().into_iter().next() else {
                    cancel_transfer(&mut model);
                    return (model, Effect::None);
                };
                let new_name = model.rename_input.text.clone();
                let dst = target
                    .path
                    .parent()
                    .map_or_else(|| PathBuf::from(&new_name), |p| p.join(&new_name));
                let kind = PendingKind::MoveRename(target.path, dst);
                if let Some(message) = same_location_message(&kind) {
                    // Leave the dialog open behind the message, so the name can
                    // be edited rather than typed again from scratch.
                    model.error_message = Some(message);
                    return (model, Effect::None);
                }
                model.rename_input.close();
                model.transfer_mode = TransferMode::None;
                model.pending_select = Some(new_name);
                let effect = begin_transfer(&mut model, kind);
                return (model, effect);
            }
            // Deactivate the dialog (keep text) and open the destination panel.
            model.rename_input.active = false;
            let mode = model.transfer_mode;
            open_target_panel(&mut model, mode);
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
    // Focus is not touched here: a copy or move already handed it back to the
    // source panel when the destination was confirmed, and a delete never left
    // the panel it ran in.
    model.left_files.reload_keeping_selection();
    model.right_files.reload_keeping_selection();
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

fn origin_file_panel(model: &Model) -> &file_panel::Model {
    match model.origin_panel {
        ActivePanel::RightFiles => &model.right_files,
        ActivePanel::LeftFiles | ActivePanel::Pinned => &model.left_files,
    }
}

/// Apply an edit to whichever of a result panel's two input fields has the
/// focus, then report the root, query and mask the search should re-run with.
/// `None` while the query is empty — a mask on its own has nothing to look for.
fn edit_query_panel<T>(
    panel: Option<&mut ResultPanel<T>>,
    edit: impl FnOnce(&mut input_box::Model),
) -> Option<(PathBuf, String, String)> {
    let panel = panel?;
    match panel.input_field {
        InputField::Query => edit(&mut panel.query),
        InputField::Mask => edit(&mut panel.mask),
    }
    panel.results.clear();
    panel.selection = 0;
    panel.done = panel.query.text.is_empty();
    (!panel.query.text.is_empty()).then(|| {
        (
            panel.root.clone(),
            panel.query.text.clone(),
            panel.mask.text.clone(),
        )
    })
}

/// Move the cursor within the focused field, crossing into the other one when
/// it runs off the end: Right past the query enters the mask at its start, Left
/// before the mask returns to the end of the query.
fn move_query_cursor<T>(panel: Option<&mut ResultPanel<T>>, right: bool) {
    let Some(panel) = panel else { return };
    match (panel.input_field, right) {
        (InputField::Query, true) if panel.query.cursor() == panel.query.text.len() => {
            panel.input_field = InputField::Mask;
            panel.mask.cursor_home();
        }
        (InputField::Mask, false) if panel.mask.cursor() == 0 => {
            panel.input_field = InputField::Query;
            panel.query.cursor_end();
        }
        (InputField::Query, true) => panel.query.move_right(),
        (InputField::Query, false) => panel.query.move_left(),
        (InputField::Mask, true) => panel.mask.move_right(),
        (InputField::Mask, false) => panel.mask.move_left(),
    }
}

/// Apply an edit to a result panel's query row. Cursor moves can cross between
/// the query and the mask, so they take the crossing path; text edits go
/// through `edit_query_panel`, which also reports the re-run it needs.
fn edit_result_panel<T>(
    panel: Option<&mut ResultPanel<T>>,
    op: EditOp,
) -> Option<(PathBuf, String, String)> {
    match op {
        EditOp::Char(c) => edit_query_panel(panel, |field| field.insert(c)),
        EditOp::Backspace => edit_query_panel(panel, input_box::Model::backspace),
        EditOp::CursorLeft => {
            move_query_cursor(panel, false);
            None
        }
        EditOp::CursorRight => {
            move_query_cursor(panel, true);
            None
        }
    }
}

/// What a step on a result popup leaves for the caller to do, once the panel
/// itself has been updated. Keeps `step_search_panel` generic over the result
/// type while the model slot and the search effect stay per-kind.
enum PanelOutcome {
    Nothing,
    /// The query row changed; re-run the search over this root, query and mask.
    Rerun(PathBuf, String, String),
    /// A result was picked; close the popup and move the file list onto it.
    Reveal(PathBuf),
    /// The popup was dismissed.
    Close,
}

/// Apply a message to whichever result popup is open. Both popups are the same
/// `ResultPanel`, so this is written once and instantiated twice.
fn step_search_panel<T: Located>(slot: Option<&mut ResultPanel<T>>, msg: Message) -> PanelOutcome {
    let Some(panel) = slot else {
        return PanelOutcome::Nothing;
    };
    match msg {
        Message::Edit(_, op) => match edit_result_panel(Some(panel), op) {
            Some((root, query, mask)) => PanelOutcome::Rerun(root, query, mask),
            None => PanelOutcome::Nothing,
        },
        Message::SearchToggleFocus(_) => {
            panel.input_focused = !panel.input_focused;
            PanelOutcome::Nothing
        }
        Message::Nav(_, op) => {
            match op {
                // Up off the first result returns the keys to the query row.
                NavOp::Up => {
                    if panel.selection == 0 {
                        panel.input_focused = true;
                    } else {
                        panel.selection -= 1;
                    }
                }
                NavOp::Down if !panel.results.is_empty() => {
                    panel.selection = (panel.selection + 1).min(panel.results.len() - 1);
                }
                // The results list neither pages nor marks.
                _ => {}
            }
            PanelOutcome::Nothing
        }
        Message::Close(_) => PanelOutcome::Close,
        // Enter with nothing to confirm leaves the popup up.
        Message::SearchConfirm(_) => panel
            .results
            .get(panel.selection)
            .map_or(PanelOutcome::Nothing, |r| {
                PanelOutcome::Reveal(r.path().to_path_buf())
            }),
        _ => PanelOutcome::Nothing,
    }
}

/// Move the left file list onto `path` and give it the focus, so a confirmed
/// result is the entry under the cursor.
fn reveal(model: &mut Model, path: &Path) {
    if let Some(dir) = path.parent().map(Path::to_path_buf) {
        let name = path
            .file_name()
            .map(|n: &std::ffi::OsStr| n.to_string_lossy().into_owned());
        model.left_files.navigate_to(dir);
        let pos = name.and_then(|name| {
            model
                .left_files
                .visible_entries()
                .position(|(_, e)| e.name == name)
        });
        if let Some(pos) = pos {
            model.left_files.selection = pos;
        }
    }
    model.active_panel = ActivePanel::LeftFiles;
}

fn update_search(mut model: Model, kind: SearchKind, msg: Message) -> (Model, Effect) {
    // Opening needs the root before there is a panel to ask for one.
    if matches!(msg, Message::SearchOpen(_)) {
        let Some(root) = active_panel_dir(&model) else {
            return (model, Effect::None);
        };
        // Index up front so the first keystroke searches a ready index.
        return match kind {
            SearchKind::Content => {
                model.content_search = Some(ResultPanel::new(root.clone()));
                (model, Effect::PrepareContentSearch { root })
            }
            SearchKind::Files => {
                model.file_find = Some(ResultPanel::new(root.clone()));
                (model, Effect::PrepareFileFind { root })
            }
        };
    }

    let outcome = match kind {
        SearchKind::Content => step_search_panel(model.content_search.as_mut(), msg),
        SearchKind::Files => step_search_panel(model.file_find.as_mut(), msg),
    };

    match outcome {
        PanelOutcome::Nothing => (model, Effect::None),
        PanelOutcome::Rerun(root, query, mask) => {
            let effect = match kind {
                SearchKind::Content => Effect::StartContentSearch { root, query, mask },
                SearchKind::Files => Effect::StartFileFind { root, query, mask },
            };
            (model, effect)
        }
        PanelOutcome::Close | PanelOutcome::Reveal(_) => {
            match kind {
                SearchKind::Content => model.content_search = None,
                SearchKind::Files => model.file_find = None,
            }
            if let PanelOutcome::Reveal(path) = outcome {
                reveal(&mut model, &path);
            }
            (model, Effect::None)
        }
    }
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
        Message::Nav(_, op) => update_command_picker_move(model, op),
        Message::Close(_) => {
            model.command_picker = None;
            (model, Effect::None)
        }
        Message::CommandPickerConfirm => update_command_picker_confirm(model),
        Message::CommandPickerShortcut(c) => update_command_picker_shortcut(model, c),
        Message::Edit(_, op) => {
            if let Some(input) = command_input_mut(&mut model) {
                input_box::apply(input, op);
            }
            (model, Effect::None)
        }
        Message::Cancel(_) => {
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

fn update_capture_view(mut model: Model, msg: Message) -> (Model, Effect) {
    let Some(p) = &mut model.capture_view else {
        return (model, Effect::None);
    };
    let max = capture_view::max_scroll(p);
    let page = capture_view::page_step(p);
    match msg {
        Message::Nav(_, op) => {
            p.scroll = match op {
                NavOp::Up => p.scroll.saturating_sub(1),
                NavOp::Down => p.scroll.saturating_add(1).min(max),
                NavOp::PageUp => p.scroll.saturating_sub(page),
                NavOp::PageDown => p.scroll.saturating_add(page).min(max),
                // Captured output has nothing to mark.
                NavOp::MarkUp | NavOp::MarkDown => p.scroll,
            };
        }
        Message::Close(_) => {
            model.capture_view = None;
            refresh_both_panels(&mut model);
        }
        _ => {}
    }
    (model, Effect::None)
}

/// Upper bound on the size of a file the viewer will load into memory.
const MAX_VIEW_BYTES: u64 = 5 * 1024 * 1024;

/// The entry highlighted in the active file panel: its path, display name and
/// whether it is a directory.
fn view_target(model: &Model) -> Option<(PathBuf, String, bool)> {
    let panel = match model.active_panel {
        ActivePanel::LeftFiles => &model.left_files,
        ActivePanel::RightFiles => &model.right_files,
        ActivePanel::Pinned => return None,
    };
    let (_, entry) = panel.visible_entries().nth(panel.selection)?;
    Some((
        panel.current_dir.join(&entry.name),
        entry.name.clone(),
        entry.is_dir,
    ))
}

/// Build the viewer contents for one entry. A directory or an oversized file is
/// not an error here — the viewer is a live preview of whatever the file list
/// points at, so it shows the reason as its text instead. Binary content is not
/// a problem at all: the registry hands it to the hex view.
fn load_file_view(model: &Model, path: PathBuf, name: String, is_dir: bool) -> FileView {
    if !is_dir && let Some(content) = load_image_view(model, &path) {
        return FileView {
            name,
            path,
            content,
        };
    }
    let bytes = if is_dir {
        Err("directory".to_owned())
    } else {
        read_file(&path)
    };
    let state = match bytes {
        // The registry picks by content first, so a binary file lands in the hex
        // view whatever it is named. Nothing matching at all — a text file with
        // an unknown extension — still gets the plain-text view.
        Ok(bytes) => {
            let view = model
                .view_registry
                .find_for(&path, &bytes)
                .unwrap_or_else(|| Arc::new(PlainTextView::new()));
            ViewState::from_bytes(bytes, view)
        }
        Err(reason) => ViewState::new(
            format!("<{reason}>"),
            Arc::new(PlainTextView::new()) as Arc<dyn FormatView>,
        ),
    };
    FileView {
        name,
        path,
        content: ViewContent::Text(state),
    }
}

/// Start decoding `path` as an image, or return `None` when it is not one the
/// terminal can be asked to draw: an unrecognised extension, or no picker (no
/// terminal was queried). `None` sends the entry down the text path, which
/// reports the reason itself. Decoding runs on the view's worker thread, so
/// this returns before the image is available.
fn load_image_view(model: &Model, path: &std::path::Path) -> Option<ViewContent> {
    let picker = model.picker.as_ref()?;
    image_view::open(picker, path).map(|view| ViewContent::Image(Box::new(view)))
}

/// `v` toggles the viewer panel: it closes an open viewer, and otherwise opens
/// one on the highlighted entry.
fn update_view_file(mut model: Model) -> (Model, Effect) {
    if model.file_view.is_some() {
        close_file_view(&mut model);
        return (model, Effect::None);
    }
    let Some((path, name, is_dir)) = view_target(&model) else {
        return (model, Effect::None);
    };
    model.file_view = Some(load_file_view(&model, path, name, is_dir));
    (model, Effect::None)
}

fn close_file_view(model: &mut Model) {
    model.file_view = None;
    model.file_view_focused = false;
}

/// Reload the open viewer when the file list has moved to a different entry.
fn sync_file_view(model: &mut Model) {
    let Some(current) = model.file_view.as_ref().map(|v| v.path.clone()) else {
        return;
    };
    let Some((path, name, is_dir)) = view_target(model) else {
        return;
    };
    if path != current {
        model.file_view = Some(load_file_view(model, path, name, is_dir));
    }
}

/// Read `path` for the viewer, rejecting only files too big to hold in memory.
/// Binary content is fine: the hex view renders it.
///
/// An archive is exempt from the size limit: its listing is built from the
/// central directory alone, so even a multi-gigabyte one is worth opening. The
/// bytes are still read whole, which is the cost of that exemption.
fn read_file(path: &std::path::Path) -> Result<Vec<u8>, String> {
    let len = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if len > MAX_VIEW_BYTES && !ZipView::new().matches(path) {
        return Err(format!("file too large to view ({len} bytes)"));
    }
    std::fs::read(path).map_err(|e| e.to_string())
}

fn update_file_view(mut model: Model, msg: Message) -> (Model, Effect) {
    let Some(v) = &mut model.file_view else {
        return (model, Effect::None);
    };
    // An image is always drawn to fit the panel, so scrolling has nothing to do.
    let text = match &mut v.content {
        ViewContent::Text(state) => Some(state),
        ViewContent::Image(_) => None,
    };
    match (msg, text) {
        (Message::Nav(_, op), Some(state)) => match op {
            NavOp::Up => state.scroll_up(1),
            NavOp::Down => state.scroll_down(1),
            NavOp::PageUp => state.page_up(),
            NavOp::PageDown => state.page_down(),
            // A viewed file has nothing to mark.
            NavOp::MarkUp | NavOp::MarkDown => {}
        },
        (Message::Close(_), _) => {
            close_file_view(&mut model);
        }
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

fn update_command_picker_move(mut model: Model, op: NavOp) -> (Model, Effect) {
    if let Some(cp) = &mut model.command_picker
        && cp.input.is_none()
        && !cp.presets.is_empty()
    {
        match op {
            NavOp::Up => cp.selection = cp.selection.saturating_sub(1),
            NavOp::Down => cp.selection = (cp.selection + 1).min(cp.presets.len() - 1),
            // The preset list neither pages nor marks.
            _ => {}
        }
    }
    (model, Effect::None)
}

/// A character typed with the preset list up. A preset bound to it runs
/// straight away — an explicit binding outranks the built-in `j`/`k`, which
/// stay available on the arrow keys. Unbound characters fall back to
/// navigation, so `j`/`k` keep working when nothing claims them.
fn update_command_picker_shortcut(mut model: Model, c: char) -> (Model, Effect) {
    let Some(cp) = &mut model.command_picker else {
        return (model, Effect::None);
    };
    if let Some(index) = cp.shortcut_index(c) {
        cp.selection = index;
        return update_command_picker_confirm(model);
    }
    match c {
        'j' => update_command_picker_move(model, NavOp::Down),
        'k' => update_command_picker_move(model, NavOp::Up),
        _ => (model, Effect::None),
    }
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
