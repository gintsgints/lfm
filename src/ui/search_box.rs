use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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
        Message::ConfirmFilter | Message::FilterBarDown => {
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

pub fn render(frame: &mut Frame, area: Rect, model: &Model) {
    let border_style = if model.active {
        Style::default().fg(theme::active_border())
    } else {
        Style::default().fg(theme::inactive_border())
    };

    let text_style = Style::default()
        .fg(theme::text())
        .add_modifier(Modifier::BOLD);
    let cursor_style = text_style.add_modifier(Modifier::UNDERLINED);

    let content = if model.active {
        Line::from(vec![
            Span::styled(model.text.clone(), text_style),
            Span::styled("_", cursor_style),
        ])
    } else {
        Line::from(Span::styled(model.text.clone(), text_style))
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(" filter ", Style::default().fg(theme::text())));

    frame.render_widget(Paragraph::new(content).block(block), area);
}

pub fn title(path_label: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {path_label} "),
        Style::default().fg(theme::text()),
    ))
}
