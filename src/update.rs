use crate::message::Message;
use crate::model::{ActivePanel, Model};
use crate::ui::{file_panel, pinned_panel};

pub enum Effect {
    None,
    Quit,
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
        msg => {
            match model.active_panel {
                ActivePanel::LeftFiles => {
                    model.left_files = file_panel::update(model.left_files, msg);
                }
                ActivePanel::RightFiles => {
                    model.right_files = file_panel::update(model.right_files, msg);
                }
                ActivePanel::Pinned => {
                    model.pinned_panel = pinned_panel::update(model.pinned_panel, msg);
                }
            }
            (model, Effect::None)
        }
    }
}

fn origin_file_panel(model: &Model) -> &file_panel::Model {
    match model.origin_panel {
        ActivePanel::RightFiles => &model.right_files,
        ActivePanel::LeftFiles | ActivePanel::Pinned => &model.left_files,
    }
}
