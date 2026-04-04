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

        let in_filter = match model.active_panel {
            ActivePanel::LeftFiles => model.left_files.search.active,
            ActivePanel::RightFiles => model.right_files.search.active,
            ActivePanel::Pinned => false,
        };
        let in_new_path = match model.active_panel {
            ActivePanel::LeftFiles => model.left_files.new_path_input.active,
            ActivePanel::RightFiles => model.right_files.new_path_input.active,
            ActivePanel::Pinned => false,
        };

        if let Some(msg) = to_message(&event::read()?, model.active_panel, in_filter, in_new_path) {
            let (next_model, effect) = update(model, msg);
            model = next_model;
            if matches!(effect, Effect::Quit) {
                let _ = state::save(&model.to_persisted());
                return Ok(());
            }
        }
    }
}

fn to_message(
    event: &Event,
    active_panel: ActivePanel,
    in_filter: bool,
    in_new_path: bool,
) -> Option<Message> {
    if let Event::Key(key) = event {
        if in_new_path {
            return match key.code {
                KeyCode::Esc => Some(Message::NewPathCancel),
                KeyCode::Enter => Some(Message::NewPathConfirm),
                KeyCode::Backspace => Some(Message::NewPathBackspace),
                KeyCode::Char(c) => Some(Message::NewPathChar(c)),
                _ => None,
            };
        }

        if in_filter {
            return match key.code {
                KeyCode::Esc => Some(Message::ExitFilter),
                KeyCode::Enter => Some(Message::ConfirmFilter),
                KeyCode::Backspace => Some(Message::FilterBackspace),
                KeyCode::Char(c) => Some(Message::FilterChar(c)),
                _ => None,
            };
        }

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
            KeyCode::Char('/') => Some(Message::EnterFilter),
            KeyCode::Char('n') => Some(Message::NewPath),
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
