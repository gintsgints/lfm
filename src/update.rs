use crate::message::Message;
use crate::model::{ActivePanel, Model};
use crate::ui::file_panel;

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
        msg => {
            match model.active_panel {
                ActivePanel::LeftFiles => {
                    model.left_files = file_panel::update(model.left_files, msg);
                }
                ActivePanel::RightFiles => {
                    model.right_files = file_panel::update(model.right_files, msg);
                }
            }
            (model, Effect::None)
        }
    }
}
