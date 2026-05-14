use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::message::Message;
use crate::theme;

pub struct Model {
    pub text: String,
    pub active: bool,
}

impl Model {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            active: false,
        }
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.active = false;
    }

    pub fn is_filtering(&self) -> bool {
        self.active || !self.text.is_empty()
    }
}

pub fn update(mut model: Model, msg: Message) -> (Model, bool) {
    let mut reset_selection = false;
    match msg {
        Message::EnterFilter => {
            model.active = true;
        }
        Message::FilterChar(c) => {
            model.text.push(c);
            reset_selection = true;
        }
        Message::FilterBackspace => {
            model.text.pop();
            reset_selection = true;
        }
        Message::ConfirmFilter => {
            model.active = false;
        }
        Message::ExitFilter => {
            model.text.clear();
            model.active = false;
            reset_selection = true;
        }
        _ => {}
    }
    (model, reset_selection)
}

pub fn title(model: &Model, path_label: &str) -> Line<'static> {
    if model.is_filtering() {
        Line::from(vec![
            Span::styled(" 🔍 ", Style::default().fg(theme::ACTIVE_BORDER)),
            Span::styled(model.text.clone(), Style::default().fg(theme::TEXT)),
            Span::raw(" "),
        ])
    } else {
        Line::from(Span::styled(
            format!(" {path_label} "),
            Style::default().fg(theme::TEXT),
        ))
    }
}
