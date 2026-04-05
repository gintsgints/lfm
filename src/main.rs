use std::{io, path::PathBuf};

use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
};

mod archive;
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
    let choosedir = std::env::var_os("LFM_CHOOSEDIR").map(PathBuf::from);
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    let dir = result?;
    if let Some(path) = choosedir {
        let _ = std::fs::write(path, dir.display().to_string());
    }
    Ok(())
}

fn run(mut terminal: DefaultTerminal) -> io::Result<PathBuf> {
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
        let in_delete_confirm = match model.active_panel {
            ActivePanel::LeftFiles => model.left_files.delete_confirm,
            ActivePanel::RightFiles => model.right_files.delete_confirm,
            ActivePanel::Pinned => false,
        };

        let mode = if model.show_help {
            InputMode::Help
        } else if in_delete_confirm {
            InputMode::DeleteConfirm
        } else if in_new_path {
            InputMode::NewPath
        } else if model.copy_mode {
            InputMode::Copy
        } else if in_filter {
            InputMode::Filter
        } else {
            InputMode::Normal
        };

        if let Some(msg) = to_message(&event::read()?, model.active_panel, &mode) {
            let (next_model, effect) = update(model, msg);
            model = next_model;
            match effect {
                Effect::Quit => {
                    let _ = state::save(&model.to_persisted());
                    return Ok(model.left_files.current_dir.clone());
                }
                Effect::OpenEditor(path) => {
                    if let Some(editor) = std::env::var_os("EDITOR") {
                        ratatui::restore();
                        let _ = std::process::Command::new(editor).arg(&path).status();
                        terminal = ratatui::init();
                    }
                }
                Effect::OpenDefault(path) => {
                    #[cfg(target_os = "macos")]
                    let _ = std::process::Command::new("open").arg(&path).spawn();
                    #[cfg(target_os = "windows")]
                    let _ = std::process::Command::new("cmd")
                        .args(["/c", "start", "", &path.to_string_lossy()])
                        .spawn();
                    #[cfg(target_os = "linux")]
                    let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
                }
                Effect::None => {}
            }
        }
    }
}

enum InputMode {
    Normal,
    Filter,
    NewPath,
    DeleteConfirm,
    Copy,
    Help,
}

fn to_message(event: &Event, active_panel: ActivePanel, mode: &InputMode) -> Option<Message> {
    if let Event::Key(key) = event {
        match mode {
            InputMode::Help => {
                return match key.code {
                    KeyCode::Esc | KeyCode::Char('?') => Some(Message::ToggleHelp),
                    _ => None,
                };
            }
            InputMode::DeleteConfirm => {
                return match key.code {
                    KeyCode::Enter => Some(Message::DeleteConfirm),
                    KeyCode::Esc => Some(Message::DeleteCancel),
                    _ => None,
                };
            }
            InputMode::NewPath => {
                return match key.code {
                    KeyCode::Esc => Some(Message::NewPathCancel),
                    KeyCode::Enter => Some(Message::NewPathConfirm),
                    KeyCode::Backspace => Some(Message::NewPathBackspace),
                    KeyCode::Char(c) => Some(Message::NewPathChar(c)),
                    _ => None,
                };
            }
            InputMode::Copy => {
                if key.code == KeyCode::Esc {
                    return Some(Message::CancelCopy);
                }
                if key.code == KeyCode::Enter && active_panel == ActivePanel::RightFiles {
                    return Some(Message::ConfirmCopy);
                }
            }
            InputMode::Filter => {
                return match key.code {
                    KeyCode::Esc => Some(Message::ExitFilter),
                    KeyCode::Enter => Some(Message::ConfirmFilter),
                    KeyCode::Backspace => Some(Message::FilterBackspace),
                    KeyCode::Char(c) => Some(Message::FilterChar(c)),
                    _ => None,
                };
            }
            InputMode::Normal => {}
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
            KeyCode::Char('?') => Some(Message::ToggleHelp),
            KeyCode::Char('s') if active_panel != ActivePanel::Pinned => Some(Message::CycleSort),
            KeyCode::Char('z') if active_panel != ActivePanel::Pinned => Some(Message::ZipFiles),
            KeyCode::Char('u') if active_panel != ActivePanel::Pinned => Some(Message::UnzipFile),
            KeyCode::Char('e') if active_panel != ActivePanel::Pinned => Some(Message::OpenEditor),
            KeyCode::Char('o') if active_panel != ActivePanel::Pinned => Some(Message::OpenDefault),
            KeyCode::Char('c') if active_panel != ActivePanel::Pinned => Some(Message::StartCopy),
            KeyCode::Char('d') if active_panel != ActivePanel::Pinned => Some(Message::DeleteFiles),
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
