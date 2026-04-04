use std::io;

use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
};

mod message;
mod model;
mod state;
mod theme;
mod ui;
mod update;
mod view;

use message::Message;
use model::{ActivePanel, Model};
use update::{Effect, update};
use view::view;

fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal) -> io::Result<()> {
    let mut model = Model::init(state::load())?;

    loop {
        terminal.draw(|frame| view(&model, frame))?;

        if let Some(msg) = to_message(&event::read()?, model.active_panel) {
            let (next_model, effect) = update(model, msg);
            model = next_model;
            if matches!(effect, Effect::Quit) {
                let _ = state::save(&model.to_persisted());
                return Ok(());
            }
        }
    }
}

fn to_message(event: &Event, active_panel: ActivePanel) -> Option<Message> {
    if let Event::Key(key) = event {
        match key.code {
            KeyCode::Char('q') => Some(Message::Quit),
            KeyCode::Tab => Some(Message::NextPanel),
            KeyCode::BackTab => Some(Message::PrevPanel),
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                Some(Message::MarkSelectUp)
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                Some(Message::MarkSelectDown)
            }
            KeyCode::Char('K') => Some(Message::MarkSelectUp),
            KeyCode::Char('J') => Some(Message::MarkSelectDown),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::SelectUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::SelectDown),
            KeyCode::Left | KeyCode::Char('h') => Some(Message::DirUp),
            KeyCode::Right | KeyCode::Char('l') => Some(Message::DirEnter),
            KeyCode::Char('p') if active_panel == ActivePanel::Pinned => {
                Some(Message::PinCurrentDir)
            }
            KeyCode::Char('d') if active_panel == ActivePanel::Pinned => {
                Some(Message::DeletePinnedDir)
            }
            KeyCode::Char('p') => Some(Message::TogglePinnedPanel),
            KeyCode::Enter | KeyCode::Char(' ') if active_panel == ActivePanel::Pinned => {
                Some(Message::SelectPinnedDir)
            }
            KeyCode::Esc if active_panel == ActivePanel::Pinned => Some(Message::TogglePinnedPanel),
            KeyCode::Esc => Some(Message::ClearSelection),
            _ => None,
        }
    } else {
        None
    }
}
