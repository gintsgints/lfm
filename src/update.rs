use crate::message::Message;
use crate::model::{ActivePanel, Model};

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
        Message::SelectUp => {
            if model.active_panel == ActivePanel::Files {
                model.selection = model.selection.saturating_sub(1);
            }
            (model, Effect::None)
        }
        Message::SelectDown => {
            if model.active_panel == ActivePanel::Files {
                let count = model.entry_count();
                if count > 0 {
                    model.selection = (model.selection + 1).min(count - 1);
                }
            }
            (model, Effect::None)
        }
    }
}
