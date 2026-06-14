use std::{io, path::PathBuf, sync::mpsc, time::Duration};

use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
};

mod archive;
pub mod debug;
mod file_find;
mod message;
mod model;
mod search;
mod state;
#[cfg(test)]
mod tests;
mod theme;
mod transfer;
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
    enable_extended_key_reporting();
    let result = run(terminal);
    disable_extended_key_reporting();
    ratatui::restore();
    let dir = result?;
    if let Some(path) = choosedir {
        let _ = std::fs::write(path, dir.display().to_string());
    }
    Ok(())
}

fn enable_extended_key_reporting() {
    use ratatui::crossterm::{
        event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
        execute,
    };
    #[cfg(feature = "debug")]
    debug_log!(
        "kbd enhancement query: {:?}",
        ratatui::crossterm::terminal::supports_keyboard_enhancement()
    );
    let _ = execute!(
        io::stdout(),
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        )
    );
}

fn disable_extended_key_reporting() {
    use ratatui::crossterm::{event::PopKeyboardEnhancementFlags, execute};
    let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
}

#[cfg(feature = "debug")]
fn log_key_and_is_release(event: &Event) -> bool {
    let Event::Key(key) = event else {
        return false;
    };
    #[cfg(feature = "debug")]
    debug_log!("key: {:?} {:?} {:?}", key.code, key.modifiers, key.kind);
    key.kind == KeyEventKind::Release
}

fn open_in_editor(terminal: &mut DefaultTerminal, path: &std::path::Path) {
    let Some(editor) = std::env::var_os("EDITOR") else {
        return;
    };
    disable_extended_key_reporting();
    ratatui::restore();
    let _ = std::process::Command::new(editor).arg(path).status();
    *terminal = ratatui::init();
    enable_extended_key_reporting();
}

fn run(mut terminal: DefaultTerminal) -> io::Result<PathBuf> {
    let mut model = Model::init(state::load())?;
    let mut progress_rx: Option<mpsc::Receiver<transfer::ProgressMsg>> = None;
    let mut search_rx: Option<mpsc::Receiver<search::SearchMsg>> = None;
    let mut file_find_rx: Option<mpsc::Receiver<file_find::FileFindMsg>> = None;

    loop {
        terminal.draw(|frame| view(&model, frame))?;

        let (m, got_progress) = drain_progress(model, &mut progress_rx);
        model = m;
        let (m, got_search) = drain_search(model, &mut search_rx);
        model = m;
        let (m, got_file_find) = drain_file_find(model, &mut file_find_rx);
        model = m;

        // Decide how long to wait for input.
        //
        // When we just drained progress/search results, redraw promptly with a
        // zero-timeout poll — but still service the keyboard so a fast-producing
        // background thread (e.g. a search matching many lines) can't starve
        // input and freeze the UI. When a thread is running but idle, poll with
        // a short timeout. Otherwise block until the next event.
        let event = if got_progress || got_search || got_file_find {
            if event::poll(Duration::ZERO)? {
                Some(event::read()?)
            } else {
                None
            }
        } else if progress_rx.is_some() || search_rx.is_some() || file_find_rx.is_some() {
            if event::poll(Duration::from_millis(50))? {
                Some(event::read()?)
            } else {
                None
            }
        } else {
            Some(event::read()?)
        };

        let Some(event) = event else { continue };

        #[cfg(feature = "debug")]
        if log_key_and_is_release(&event) {
            continue;
        }

        let mode = input_mode(&model);

        if let Some(msg) = to_message(&event, model.active_panel, &mode) {
            let (next_model, effect) = update(model, msg);
            model = next_model;
            match effect {
                Effect::Quit => {
                    let _ = state::save(&model.to_persisted());
                    return Ok(model.left_files.current_dir.clone());
                }
                Effect::OpenEditor(path) => open_in_editor(&mut terminal, &path),
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
                Effect::StartCopy(sources, dst) => {
                    let (tx, rx) = mpsc::channel();
                    progress_rx = Some(rx);
                    std::thread::spawn(move || transfer::run_copy(&sources, &dst, &tx));
                }
                Effect::StartMove(sources, dst) => {
                    let (tx, rx) = mpsc::channel();
                    progress_rx = Some(rx);
                    std::thread::spawn(move || transfer::run_move(&sources, &dst, &tx));
                }
                Effect::StartCopyRename(src, dst) => {
                    let (tx, rx) = mpsc::channel();
                    progress_rx = Some(rx);
                    std::thread::spawn(move || transfer::run_copy_rename(&src, &dst, &tx));
                }
                Effect::StartMoveRename(src, dst) => {
                    let (tx, rx) = mpsc::channel();
                    progress_rx = Some(rx);
                    std::thread::spawn(move || transfer::run_move_rename(&src, &dst, &tx));
                }
                Effect::StartDelete(sources) => {
                    let (tx, rx) = mpsc::channel();
                    progress_rx = Some(rx);
                    std::thread::spawn(move || transfer::run_delete(&sources, &tx));
                }
                Effect::StartContentSearch { root, query } => {
                    let (tx, rx) = mpsc::channel();
                    search_rx = Some(rx);
                    std::thread::spawn(move || search::run_search(&root, &query, &tx));
                }
                Effect::StartFileFind { root, query } => {
                    let (tx, rx) = mpsc::channel();
                    file_find_rx = Some(rx);
                    std::thread::spawn(move || file_find::run_file_find(&root, &query, &tx));
                }
                Effect::None => {}
            }
        }
    }
}

fn drain_progress(
    mut model: Model,
    progress_rx: &mut Option<mpsc::Receiver<transfer::ProgressMsg>>,
) -> (Model, bool) {
    let mut got_progress = false;
    loop {
        let result = match progress_rx.as_ref() {
            None => break,
            Some(rx) => rx.try_recv(),
        };
        match result {
            Ok(transfer::ProgressMsg::Tick { current, total }) => {
                let (m, _) = update(model, Message::ProgressTick { current, total });
                model = m;
                got_progress = true;
            }
            Ok(transfer::ProgressMsg::Done { error }) => {
                let (m, _) = update(model, Message::ProgressDone);
                model = m;
                if let Some(err) = error {
                    model.error_message = Some(err);
                }
                *progress_rx = None;
                got_progress = true;
                break;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                let (m, _) = update(model, Message::ProgressDone);
                model = m;
                *progress_rx = None;
                got_progress = true;
                break;
            }
            Err(mpsc::TryRecvError::Empty) => break,
        }
    }
    (model, got_progress)
}

fn drain_search(
    mut model: Model,
    search_rx: &mut Option<mpsc::Receiver<search::SearchMsg>>,
) -> (Model, bool) {
    if model.content_search.is_none() {
        *search_rx = None;
        return (model, false);
    }
    let mut got_result = false;
    loop {
        let result = match search_rx.as_ref() {
            None => break,
            Some(rx) => rx.try_recv(),
        };
        match result {
            Ok(search::SearchMsg::Hit(r)) => {
                if let Some(cs) = &mut model.content_search {
                    cs.results.push(r);
                }
                got_result = true;
            }
            Ok(search::SearchMsg::Done) | Err(mpsc::TryRecvError::Disconnected) => {
                if let Some(cs) = &mut model.content_search {
                    cs.done = true;
                }
                *search_rx = None;
                got_result = true;
                break;
            }
            Err(mpsc::TryRecvError::Empty) => break,
        }
    }
    (model, got_result)
}

fn drain_file_find(
    mut model: Model,
    file_find_rx: &mut Option<mpsc::Receiver<file_find::FileFindMsg>>,
) -> (Model, bool) {
    if model.file_find.is_none() {
        *file_find_rx = None;
        return (model, false);
    }
    let mut got_result = false;
    loop {
        let result = match file_find_rx.as_ref() {
            None => break,
            Some(rx) => rx.try_recv(),
        };
        match result {
            Ok(file_find::FileFindMsg::Hit(r)) => {
                if let Some(ff) = &mut model.file_find {
                    ff.results.push(r);
                }
                got_result = true;
            }
            Ok(file_find::FileFindMsg::Done) | Err(mpsc::TryRecvError::Disconnected) => {
                if let Some(ff) = &mut model.file_find {
                    ff.done = true;
                }
                *file_find_rx = None;
                got_result = true;
                break;
            }
            Err(mpsc::TryRecvError::Empty) => break,
        }
    }
    (model, got_result)
}

enum InputMode {
    Normal,
    Filter,
    FilteredNormal,
    NewPath,
    GotoPath,
    DeleteConfirm,
    Copy,
    Move,
    Rename,
    Help,
    Progress,
    Error,
    ContentSearchInput,
    ContentSearchResults,
    FileFindInput,
    FileFindResults,
}

fn input_mode(model: &Model) -> InputMode {
    let active_fp = match model.active_panel {
        ActivePanel::LeftFiles => Some(&model.left_files),
        ActivePanel::RightFiles => Some(&model.right_files),
        ActivePanel::Pinned => None,
    };
    let in_filter = active_fp.is_some_and(|p| p.search.active);
    let filter_locked = active_fp.is_some_and(|p| !p.search.active && !p.search.text.is_empty());
    let in_new_path = active_fp.is_some_and(|p| p.new_path_input.active);
    let in_goto = active_fp.is_some_and(|p| p.goto_input.active);
    let in_delete = active_fp.is_some_and(|p| p.delete_confirm);

    if model.error_message.is_some() {
        InputMode::Error
    } else if let Some(cs) = &model.content_search {
        if cs.input_focused {
            InputMode::ContentSearchInput
        } else {
            InputMode::ContentSearchResults
        }
    } else if let Some(ff) = &model.file_find {
        if ff.input_focused {
            InputMode::FileFindInput
        } else {
            InputMode::FileFindResults
        }
    } else if model.progress.is_some() {
        InputMode::Progress
    } else if model.show_help {
        InputMode::Help
    } else if in_delete {
        InputMode::DeleteConfirm
    } else if in_new_path {
        InputMode::NewPath
    } else if in_goto {
        InputMode::GotoPath
    } else if model.rename_input.active {
        InputMode::Rename
    } else if in_filter {
        InputMode::Filter
    } else if model.transfer_mode.is_copy() {
        InputMode::Copy
    } else if model.transfer_mode.is_move() {
        InputMode::Move
    } else if filter_locked {
        InputMode::FilteredNormal
    } else {
        InputMode::Normal
    }
}

enum ModeIntercept {
    /// Mode consumed the key; caller should return this message.
    Consumed(Option<Message>),
    /// Mode did not handle this key; fall through to normal handling.
    PassThrough,
}

fn to_message(event: &Event, active_panel: ActivePanel, mode: &InputMode) -> Option<Message> {
    let Event::Key(key) = event else { return None };
    match intercept_mode(key, active_panel, mode) {
        ModeIntercept::Consumed(msg) => return msg,
        ModeIntercept::PassThrough => {}
    }
    normal_key(key, active_panel)
}

fn intercept_mode(key: &KeyEvent, active_panel: ActivePanel, mode: &InputMode) -> ModeIntercept {
    match mode {
        InputMode::Help => ModeIntercept::Consumed(match key.code {
            KeyCode::Esc | KeyCode::Char('?') => Some(Message::ToggleHelp),
            _ => None,
        }),
        InputMode::DeleteConfirm => ModeIntercept::Consumed(match key.code {
            KeyCode::Enter => Some(Message::DeleteConfirm),
            KeyCode::Esc => Some(Message::DeleteCancel),
            _ => None,
        }),
        InputMode::NewPath => ModeIntercept::Consumed(match key.code {
            KeyCode::Esc => Some(Message::NewPathCancel),
            KeyCode::Enter => Some(Message::NewPathConfirm),
            KeyCode::Backspace => Some(Message::NewPathBackspace),
            KeyCode::Left => Some(Message::NewPathCursorLeft),
            KeyCode::Right => Some(Message::NewPathCursorRight),
            KeyCode::Char(c) => Some(Message::NewPathChar(c)),
            _ => None,
        }),
        InputMode::GotoPath => ModeIntercept::Consumed(match key.code {
            KeyCode::Esc => Some(Message::GotoPathCancel),
            KeyCode::Enter => Some(Message::GotoPathConfirm),
            KeyCode::Backspace => Some(Message::GotoPathBackspace),
            KeyCode::Left => Some(Message::GotoPathCursorLeft),
            KeyCode::Right => Some(Message::GotoPathCursorRight),
            KeyCode::Char(c) => Some(Message::GotoPathChar(c)),
            _ => None,
        }),
        InputMode::Filter => ModeIntercept::Consumed(match key.code {
            KeyCode::Esc => Some(Message::ExitFilter),
            KeyCode::Enter | KeyCode::Tab => Some(Message::ConfirmFilter),
            KeyCode::Backspace => Some(Message::FilterBackspace),
            KeyCode::Down => Some(Message::FilterBarDown),
            KeyCode::Char(c) => Some(Message::FilterChar(c)),
            _ => None,
        }),
        InputMode::Copy => {
            if key.code == KeyCode::Esc {
                return ModeIntercept::Consumed(Some(Message::CancelCopy));
            }
            if key.code == KeyCode::Enter && active_panel == ActivePanel::RightFiles {
                return ModeIntercept::Consumed(Some(Message::ConfirmCopy));
            }
            ModeIntercept::PassThrough
        }
        InputMode::Move => {
            if key.code == KeyCode::Esc {
                return ModeIntercept::Consumed(Some(Message::CancelMove));
            }
            if key.code == KeyCode::Enter && active_panel == ActivePanel::RightFiles {
                return ModeIntercept::Consumed(Some(Message::ConfirmMove));
            }
            ModeIntercept::PassThrough
        }
        InputMode::Rename => ModeIntercept::Consumed(match key.code {
            KeyCode::Esc => Some(Message::CancelRename),
            KeyCode::Enter => Some(Message::ConfirmRename),
            KeyCode::Backspace => Some(Message::RenameBackspace),
            KeyCode::Left => Some(Message::RenameCursorLeft),
            KeyCode::Right => Some(Message::RenameCursorRight),
            KeyCode::Char(c) => Some(Message::RenameChar(c)),
            _ => None,
        }),
        InputMode::FilteredNormal => match key.code {
            KeyCode::Esc => ModeIntercept::Consumed(Some(Message::ExitFilter)),
            KeyCode::Tab => ModeIntercept::Consumed(Some(Message::EnterFilter)),
            _ => ModeIntercept::PassThrough,
        },
        InputMode::Normal => ModeIntercept::PassThrough,
        // Ignore all input while a transfer is running.
        InputMode::Progress => ModeIntercept::Consumed(None),
        InputMode::Error => ModeIntercept::Consumed(match key.code {
            KeyCode::Enter | KeyCode::Esc => Some(Message::DismissError),
            _ => None,
        }),
        InputMode::ContentSearchInput
        | InputMode::ContentSearchResults
        | InputMode::FileFindInput
        | InputMode::FileFindResults => intercept_search_mode(key, mode),
    }
}

/// Key handling for the content-search and file-find popups, which share the
/// same input/results layout and navigation keys.
fn intercept_search_mode(key: &KeyEvent, mode: &InputMode) -> ModeIntercept {
    ModeIntercept::Consumed(match mode {
        InputMode::ContentSearchInput => match key.code {
            KeyCode::Esc => Some(Message::ContentSearchCancel),
            KeyCode::Enter => Some(Message::ContentSearchConfirm),
            KeyCode::Tab | KeyCode::Down => Some(Message::ContentSearchToggleFocus),
            KeyCode::Backspace => Some(Message::ContentSearchBackspace),
            KeyCode::Left => Some(Message::ContentSearchCursorLeft),
            KeyCode::Right => Some(Message::ContentSearchCursorRight),
            KeyCode::Char(c) => Some(Message::ContentSearchChar(c)),
            _ => None,
        },
        InputMode::ContentSearchResults => match key.code {
            KeyCode::Esc => Some(Message::ContentSearchCancel),
            KeyCode::Enter => Some(Message::ContentSearchConfirm),
            KeyCode::Tab => Some(Message::ContentSearchToggleFocus),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::ContentSearchUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::ContentSearchDown),
            _ => None,
        },
        InputMode::FileFindInput => match key.code {
            KeyCode::Esc => Some(Message::FileFindCancel),
            KeyCode::Enter => Some(Message::FileFindConfirm),
            KeyCode::Tab | KeyCode::Down => Some(Message::FileFindToggleFocus),
            KeyCode::Backspace => Some(Message::FileFindBackspace),
            KeyCode::Left => Some(Message::FileFindCursorLeft),
            KeyCode::Right => Some(Message::FileFindCursorRight),
            KeyCode::Char(c) => Some(Message::FileFindChar(c)),
            _ => None,
        },
        InputMode::FileFindResults => match key.code {
            KeyCode::Esc => Some(Message::FileFindCancel),
            KeyCode::Enter => Some(Message::FileFindConfirm),
            KeyCode::Tab => Some(Message::FileFindToggleFocus),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::FileFindUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::FileFindDown),
            _ => None,
        },
        _ => None,
    })
}

fn normal_key(key: &KeyEvent, active_panel: ActivePanel) -> Option<Message> {
    match key.code {
        #[cfg(feature = "debug")]
        KeyCode::Char('`') => Some(Message::ToggleDebug),
        KeyCode::Char('q') => Some(Message::Quit),
        KeyCode::Tab => Some(Message::NextPanel),
        KeyCode::BackTab => Some(Message::PrevPanel),
        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Message::MarkSelectUp),
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
        KeyCode::Char('g') if active_panel != ActivePanel::Pinned => Some(Message::GotoPath),
        KeyCode::Char('s') if active_panel != ActivePanel::Pinned => Some(Message::CycleSort),
        KeyCode::Char('S') if active_panel != ActivePanel::Pinned => Some(Message::ContentSearch),
        KeyCode::Char('F') if active_panel != ActivePanel::Pinned => Some(Message::FileFind),
        KeyCode::Char('z') if active_panel != ActivePanel::Pinned => Some(Message::ZipFiles),
        KeyCode::Char('u') if active_panel != ActivePanel::Pinned => Some(Message::UnzipFile),
        KeyCode::Char('e') if active_panel != ActivePanel::Pinned => Some(Message::OpenEditor),
        KeyCode::Char('o') if active_panel != ActivePanel::Pinned => Some(Message::OpenDefault),
        KeyCode::Char('r') if active_panel != ActivePanel::Pinned => Some(Message::RenameInPlace),
        KeyCode::Char('c') if active_panel != ActivePanel::Pinned => Some(Message::StartCopy),
        KeyCode::Char('C') if active_panel != ActivePanel::Pinned => Some(Message::StartCopyRename),
        KeyCode::Char('m') if active_panel != ActivePanel::Pinned => Some(Message::StartMove),
        KeyCode::Char('M') if active_panel != ActivePanel::Pinned => Some(Message::StartMoveRename),
        KeyCode::Char('d') if active_panel != ActivePanel::Pinned => Some(Message::DeleteFiles),
        KeyCode::Char('p') if active_panel == ActivePanel::Pinned => Some(Message::PinCurrentDir),
        KeyCode::Char('d') if active_panel == ActivePanel::Pinned => Some(Message::DeletePinnedDir),
        KeyCode::Char('p') => Some(Message::TogglePinnedPanel),
        KeyCode::Enter | KeyCode::Char(' ') if active_panel == ActivePanel::Pinned => {
            Some(Message::SelectPinnedDir)
        }
        KeyCode::Enter => Some(Message::DirEnter),
        KeyCode::Esc if active_panel == ActivePanel::Pinned => Some(Message::TogglePinnedPanel),
        KeyCode::Esc => Some(Message::ClearSelection),
        _ => None,
    }
}
