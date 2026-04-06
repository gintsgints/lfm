use std::{io, path::Path};

use crate::message::Message;
use crate::model::{ActivePanel, Model};
use crate::ui::{file_panel, pinned_panel};

pub enum Effect {
    None,
    Quit,
    OpenEditor(std::path::PathBuf),
    OpenDefault(std::path::PathBuf),
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
        Message::OpenEditor => {
            let Some(path) = active_file_path(&model) else {
                return (model, Effect::None);
            };
            (model, Effect::OpenEditor(path))
        }
        Message::OpenDefault => {
            let Some(path) = active_file_path(&model) else {
                return (model, Effect::None);
            };
            (model, Effect::OpenDefault(path))
        }
        Message::StartCopy | Message::CancelCopy | Message::ConfirmCopy => {
            (update_copy(model, msg), Effect::None)
        }
        Message::StartMove | Message::CancelMove | Message::ConfirmMove => {
            (update_move(model, msg), Effect::None)
        }
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

fn update_copy(mut model: Model, msg: Message) -> Model {
    match msg {
        Message::StartCopy => {
            let start_dir = model.left_files.current_dir.clone();
            model.right_files.navigate_to(start_dir);
            model.copy_mode = true;
            model.active_panel = ActivePanel::RightFiles;
        }
        Message::CancelCopy => {
            model.copy_mode = false;
            model.active_panel = ActivePanel::LeftFiles;
        }
        Message::ConfirmCopy => {
            let dst = {
                let rf = &model.right_files;
                match rf.entries.get(rf.selection) {
                    Some(e) if e.is_dir => rf.current_dir.join(&e.name),
                    _ => rf.current_dir.clone(),
                }
            };
            let sources = model.left_files.action_targets();
            for target in &sources {
                copy_entry(&target.path, &dst).ok();
            }
            model.copy_mode = false;
            model.active_panel = ActivePanel::LeftFiles;
            let left_dir = model.left_files.current_dir.clone();
            model.left_files.navigate_to(left_dir);
        }
        _ => {}
    }
    model
}

fn update_move(mut model: Model, msg: Message) -> Model {
    match msg {
        Message::StartMove => {
            let start_dir = model.left_files.current_dir.clone();
            model.right_files.navigate_to(start_dir);
            model.move_mode = true;
            model.active_panel = ActivePanel::RightFiles;
        }
        Message::CancelMove => {
            model.move_mode = false;
            model.active_panel = ActivePanel::LeftFiles;
        }
        Message::ConfirmMove => {
            let dst = {
                let rf = &model.right_files;
                match rf.entries.get(rf.selection) {
                    Some(e) if e.is_dir => rf.current_dir.join(&e.name),
                    _ => rf.current_dir.clone(),
                }
            };
            let sources = model.left_files.action_targets();
            for target in &sources {
                move_entry(&target.path, &dst).ok();
            }
            model.move_mode = false;
            model.active_panel = ActivePanel::LeftFiles;
            let left_dir = model.left_files.current_dir.clone();
            model.left_files.navigate_to(left_dir);
        }
        _ => {}
    }
    model
}

fn move_entry(src: &Path, dst_dir: &Path) -> io::Result<()> {
    let name = src
        .file_name()
        .ok_or_else(|| io::Error::other("no file name"))?;
    let dst = dst_dir.join(name);
    if std::fs::rename(src, &dst).is_ok() {
        return Ok(());
    }
    // Cross-device fallback: copy then delete the source.
    if src.is_dir() {
        copy_dir_recursive(src, &dst)?;
        std::fs::remove_dir_all(src)
    } else {
        std::fs::copy(src, &dst).map(|_| ())?;
        std::fs::remove_file(src)
    }
}

fn copy_entry(src: &Path, dst_dir: &Path) -> io::Result<()> {
    let name = src
        .file_name()
        .ok_or_else(|| io::Error::other("no file name"))?;
    let dst = dst_dir.join(name);
    if src.is_dir() {
        copy_dir_recursive(src, &dst)
    } else {
        std::fs::copy(src, &dst).map(|_| ())
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.filter_map(std::result::Result::ok) {
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
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

fn origin_file_panel(model: &Model) -> &file_panel::Model {
    match model.origin_panel {
        ActivePanel::RightFiles => &model.right_files,
        ActivePanel::LeftFiles | ActivePanel::Pinned => &model.left_files,
    }
}
