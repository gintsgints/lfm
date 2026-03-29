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
            match model.active_panel {
                ActivePanel::Dirs => {
                    model.dir_selection = model.dir_selection.saturating_sub(1);
                }
                ActivePanel::Files => {
                    model.file_selection = model.file_selection.saturating_sub(1);
                }
                ActivePanel::Command => {}
            }
            (model, Effect::None)
        }
        Message::SelectDown => {
            match model.active_panel {
                ActivePanel::Dirs => {
                    let count = model.dir_count();
                    if count > 0 {
                        model.dir_selection = (model.dir_selection + 1).min(count - 1);
                    }
                }
                ActivePanel::Files => {
                    let count = model.file_count();
                    if count > 0 {
                        model.file_selection = (model.file_selection + 1).min(count - 1);
                    }
                }
                ActivePanel::Command => {}
            }
            (model, Effect::None)
        }
    }
}
