//! Keyboard handling: terminal protocol setup, event normalization, and the
//! mapping from key events to [`Message`]s for each input mode.

use std::io;

use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode,
};

#[cfg(feature = "debug")]
use crate::debug_log;
use crate::message::{EditOp, Field, Message, SearchKind};
use crate::model::{ActivePanel, Model};

/// Enable the Kitty keyboard protocol and report whether it is actually active.
///
/// When active, the terminal reports e.g. `Shift+s` as `Char('s')` + `SHIFT`
/// rather than `Char('S')`, so the key dispatch must normalize shifted keys
/// (see [`normalize_key_event`]). The returned flag tracks that state.
pub fn enable_extended_key_reporting() -> bool {
    use ratatui::crossterm::{
        event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
        execute,
        terminal::supports_keyboard_enhancement,
    };
    let supported = supports_keyboard_enhancement().unwrap_or(false);
    #[cfg(feature = "debug")]
    debug_log!("kbd enhancement query: {supported:?}");
    if supported {
        let _ = execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                    | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            )
        );
    }
    supported
}

pub fn disable_extended_key_reporting() {
    use ratatui::crossterm::{event::PopKeyboardEnhancementFlags, execute};
    let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
}

/// Map an unshifted key to the character its `SHIFT` variant produces on a US
/// keyboard. Letters fold to uppercase; the digit row and punctuation fold to
/// their shifted symbols (e.g. `/` -> `?`, `1` -> `!`). Returns `None` when the
/// key has no distinct shifted form.
fn shifted_char(c: char) -> Option<char> {
    if c.is_ascii_lowercase() {
        return Some(c.to_ascii_uppercase());
    }
    let shifted = match c {
        '1' => '!',
        '2' => '@',
        '3' => '#',
        '4' => '$',
        '5' => '%',
        '6' => '^',
        '7' => '&',
        '8' => '*',
        '9' => '(',
        '0' => ')',
        '`' => '~',
        '-' => '_',
        '=' => '+',
        '[' => '{',
        ']' => '}',
        '\\' => '|',
        ';' => ':',
        '\'' => '"',
        ',' => '<',
        '.' => '>',
        '/' => '?',
        _ => return None,
    };
    Some(shifted)
}

/// Under the Kitty keyboard protocol a shifted key arrives as its *unshifted*
/// `Char` with the `SHIFT` modifier set — e.g. `Char('s')` + `SHIFT` instead of
/// `Char('S')`, and `Char('/')` + `SHIFT` instead of `Char('?')` — whereas
/// legacy terminals send the shifted character directly. Fold that back to the
/// shifted character so the key dispatch can match `'S'`, `'?'`, etc. uniformly
/// (e.g. `/` and `?` stay distinct commands), and so text inputs receive the
/// glyph the user actually typed.
pub fn normalize_key_event(event: Event, extended_keys: bool) -> Event {
    if !extended_keys {
        return event;
    }
    let Event::Key(mut key) = event else {
        return event;
    };
    if key.modifiers.contains(KeyModifiers::SHIFT)
        && let KeyCode::Char(c) = key.code
        && let Some(shifted) = shifted_char(c)
    {
        key.code = KeyCode::Char(shifted);
    }
    Event::Key(key)
}

#[cfg(feature = "debug")]
pub fn log_key(event: &Event) {
    if let Event::Key(key) = event {
        debug_log!("key: {:?} {:?} {:?}", key.code, key.modifiers, key.kind);
    }
}

/// `Some(true)` when Shift was just pressed (or auto-repeated) and `Some(false)`
/// when it was released, for bare Shift key events. Returns `None` for anything
/// else. The Kitty protocol reports modifier keys as their own press/release
/// events, which is what lets the hint bar react to Shift on its own.
pub fn shift_held_change(event: &Event) -> Option<bool> {
    let Event::Key(key) = event else {
        return None;
    };
    if !matches!(
        key.code,
        KeyCode::Modifier(ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift)
    ) {
        return None;
    }
    match key.kind {
        KeyEventKind::Press | KeyEventKind::Repeat => Some(true),
        KeyEventKind::Release => Some(false),
    }
}

/// Whether the event is a key *release*. Under the Kitty protocol every press is
/// followed by a release that must not re-trigger the command a second time.
pub fn is_key_release(event: &Event) -> bool {
    matches!(event, Event::Key(key) if key.kind == KeyEventKind::Release)
}

pub enum InputMode {
    Normal,
    Filter,
    FilteredNormal,
    NewPath,
    GotoPath,
    DeleteConfirm,
    OverwriteConfirm,
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
    CommandPicker,
    CommandInput,
    CaptureView,
    FileView,
    /// Viewer panel open but not focused: only Esc is claimed (to close it),
    /// every other key falls through to the file list.
    FileViewUnfocused,
}

pub fn input_mode(model: &Model) -> InputMode {
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
    } else if model.pending_overwrite.is_some() {
        InputMode::OverwriteConfirm
    } else if model.file_view.is_some() && model.file_view_focused {
        InputMode::FileView
    } else if model.capture_view.is_some() {
        InputMode::CaptureView
    } else if let Some(cp) = &model.command_picker {
        if cp.input.is_some() {
            InputMode::CommandInput
        } else {
            InputMode::CommandPicker
        }
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
    } else if model.file_view.is_some() {
        InputMode::FileViewUnfocused
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

pub fn to_message(event: &Event, active_panel: ActivePanel, mode: &InputMode) -> Option<Message> {
    let Event::Key(key) = event else { return None };
    match intercept_mode(key, active_panel, mode) {
        ModeIntercept::Consumed(msg) => return msg,
        ModeIntercept::PassThrough => {}
    }
    normal_key(key, active_panel)
}

/// The keys every text field answers alike. Esc and Enter are left to the
/// caller — those mean something different in each field.
fn edit_key(key: &KeyEvent, field: Field) -> Option<Message> {
    let op = match key.code {
        KeyCode::Backspace => EditOp::Backspace,
        KeyCode::Left => EditOp::CursorLeft,
        KeyCode::Right => EditOp::CursorRight,
        KeyCode::Char(c) => EditOp::Char(c),
        _ => return None,
    };
    Some(Message::Edit(field, op))
}

fn intercept_mode(key: &KeyEvent, active_panel: ActivePanel, mode: &InputMode) -> ModeIntercept {
    match mode {
        InputMode::Help => ModeIntercept::Consumed(match key.code {
            KeyCode::Esc | KeyCode::Char('?' | 'q') => Some(Message::ToggleHelp),
            KeyCode::Char('j') | KeyCode::Down => Some(Message::HelpScrollDown),
            KeyCode::Char('k') | KeyCode::Up => Some(Message::HelpScrollUp),
            _ => None,
        }),
        InputMode::DeleteConfirm => ModeIntercept::Consumed(match key.code {
            KeyCode::Enter => Some(Message::DeleteConfirm),
            KeyCode::Esc => Some(Message::DeleteCancel),
            _ => None,
        }),
        InputMode::OverwriteConfirm => ModeIntercept::Consumed(match key.code {
            KeyCode::Enter => Some(Message::OverwriteConfirm),
            KeyCode::Esc => Some(Message::OverwriteCancel),
            _ => None,
        }),
        InputMode::NewPath => ModeIntercept::Consumed(match key.code {
            KeyCode::Esc => Some(Message::NewPathCancel),
            KeyCode::Enter => Some(Message::NewPathConfirm),
            _ => edit_key(key, Field::NewPath),
        }),
        InputMode::GotoPath => ModeIntercept::Consumed(match key.code {
            KeyCode::Esc => Some(Message::GotoPathCancel),
            KeyCode::Enter => Some(Message::GotoPathConfirm),
            _ => edit_key(key, Field::GotoPath),
        }),
        InputMode::Filter => ModeIntercept::Consumed(match key.code {
            KeyCode::Esc => Some(Message::ExitFilter),
            KeyCode::Enter | KeyCode::Tab => Some(Message::ConfirmFilter),
            KeyCode::Down => Some(Message::FilterBarDown),
            _ => edit_key(key, Field::Filter),
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
            _ => edit_key(key, Field::Rename),
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
        InputMode::CommandPicker | InputMode::CommandInput | InputMode::CaptureView => {
            intercept_command_mode(key, mode)
        }
        InputMode::FileView => ModeIntercept::Consumed(file_view_key(key)),
        // Esc closes the viewer from the file list too, except while the pinned
        // panel is up — there Esc still closes that panel first.
        InputMode::FileViewUnfocused => match key.code {
            KeyCode::Esc if active_panel != ActivePanel::Pinned => {
                ModeIntercept::Consumed(Some(Message::FileViewClose))
            }
            _ => ModeIntercept::PassThrough,
        },
    }
}

/// Key handling for the viewer panel while it holds the focus. Tab hands the
/// focus back to the file list; `v`, `q` and Esc close the panel.
fn file_view_key(key: &KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Tab => Some(Message::NextPanel),
        KeyCode::BackTab => Some(Message::PrevPanel),
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q' | 'v') => Some(Message::FileViewClose),
        KeyCode::Up | KeyCode::Char('k') => Some(Message::FileViewScrollUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::FileViewScrollDown),
        KeyCode::PageUp => Some(Message::FileViewPageUp),
        KeyCode::PageDown => Some(Message::FileViewPageDown),
        _ => None,
    }
}

/// Key handling for the preset-command picker, its `{input}` step, and the
/// capture-output view.
fn intercept_command_mode(key: &KeyEvent, mode: &InputMode) -> ModeIntercept {
    ModeIntercept::Consumed(match mode {
        InputMode::CommandPicker => match key.code {
            KeyCode::Esc => Some(Message::CommandPickerCancel),
            KeyCode::Enter => Some(Message::CommandPickerConfirm),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::CommandPickerUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::CommandPickerDown),
            _ => None,
        },
        InputMode::CommandInput => match key.code {
            KeyCode::Esc => Some(Message::CommandInputCancel),
            KeyCode::Enter => Some(Message::CommandInputConfirm),
            _ => edit_key(key, Field::CommandInput),
        },
        InputMode::CaptureView => match key.code {
            KeyCode::Esc | KeyCode::Enter => Some(Message::CaptureViewClose),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::CaptureViewScrollUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::CaptureViewScrollDown),
            KeyCode::PageUp => Some(Message::CaptureViewPageUp),
            KeyCode::PageDown => Some(Message::CaptureViewPageDown),
            _ => None,
        },
        _ => None,
    })
}

/// Key handling for the content-search and file-find popups, which share the
/// same input/results layout and navigation keys.
fn intercept_search_mode(key: &KeyEvent, mode: &InputMode) -> ModeIntercept {
    let (kind, on_query_row) = match mode {
        InputMode::ContentSearchInput => (SearchKind::Content, true),
        InputMode::ContentSearchResults => (SearchKind::Content, false),
        InputMode::FileFindInput => (SearchKind::Files, true),
        InputMode::FileFindResults => (SearchKind::Files, false),
        _ => return ModeIntercept::Consumed(None),
    };

    // On the query row every key that is not navigation is an edit; Down there
    // means "into the results" rather than "next result".
    if on_query_row {
        return ModeIntercept::Consumed(match key.code {
            KeyCode::Esc => Some(Message::SearchCancel(kind)),
            KeyCode::Enter => Some(Message::SearchConfirm(kind)),
            KeyCode::Tab | KeyCode::BackTab | KeyCode::Down => {
                Some(Message::SearchToggleFocus(kind))
            }
            _ => edit_key(key, Field::SearchQuery(kind)),
        });
    }

    ModeIntercept::Consumed(match key.code {
        KeyCode::Esc => Some(Message::SearchCancel(kind)),
        KeyCode::Enter => Some(Message::SearchConfirm(kind)),
        KeyCode::Tab | KeyCode::BackTab => Some(Message::SearchToggleFocus(kind)),
        KeyCode::Up | KeyCode::Char('k') => Some(Message::SearchUp(kind)),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::SearchDown(kind)),
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
        KeyCode::Char('s') if active_panel != ActivePanel::Pinned => {
            Some(Message::SearchOpen(SearchKind::Content))
        }
        KeyCode::Char('f') if active_panel != ActivePanel::Pinned => {
            Some(Message::SearchOpen(SearchKind::Files))
        }
        KeyCode::Char('S') if active_panel != ActivePanel::Pinned => Some(Message::CycleSort),
        KeyCode::Char('z') if active_panel != ActivePanel::Pinned => Some(Message::ZipFiles),
        KeyCode::Char('u') if active_panel != ActivePanel::Pinned => Some(Message::UnzipFile),
        KeyCode::Char('e') if active_panel != ActivePanel::Pinned => Some(Message::OpenEditor),
        KeyCode::Char('v') if active_panel != ActivePanel::Pinned => Some(Message::ViewFile),
        KeyCode::Char('x') if active_panel != ActivePanel::Pinned => {
            Some(Message::OpenCommandPicker)
        }
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
