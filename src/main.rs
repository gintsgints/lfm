use std::io;

use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode},
};

mod message;
mod model;
mod ui;
mod update;
mod view;

use message::Message;
use model::Model;
use update::{Effect, update};
use view::view;

fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal) -> io::Result<()> {
    let mut model = Model::init()?;

    loop {
        terminal.draw(|frame| view(&model, frame))?;

        if let Some(msg) = to_message(event::read()?) {
            let (next_model, effect) = update(model, msg);
            model = next_model;
            if matches!(effect, Effect::Quit) {
                return Ok(());
            }
        }
    }
}

fn to_message(event: Event) -> Option<Message> {
    if let Event::Key(key) = event {
        match key.code {
            KeyCode::Char('q') => Some(Message::Quit),
            KeyCode::Tab => Some(Message::NextPanel),
            KeyCode::BackTab => Some(Message::PrevPanel),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::SelectUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::SelectDown),
            _ => None,
        }
    } else {
        None
    }
}
