use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::keys::{InputMode, input_mode, to_message};
use crate::message::{Message, Surface};
use crate::model::{ActivePanel, Model};
use crate::state::PersistedState;
use crate::terminal::{self, encode_key};
use crate::ui::file_panel;
use crate::update::{Effect, update};

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lfm-terminal-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn model_in(dir: PathBuf) -> Model {
    let mut model = Model::init(PersistedState::default()).unwrap();
    model.left_files = file_panel::Model::init(dir.clone()).unwrap();
    model.right_files = file_panel::Model::init(dir).unwrap();
    model
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

/// `t` asks for a shell in the directory the active panel is showing — not in
/// lfm's own working directory, which is what a plain `SHELL` spawn would give.
#[test]
fn t_opens_a_shell_in_the_directory_being_browsed() {
    let dir = temp_dir();
    let model = model_in(dir.clone());

    let (_, effect) = update(model, Message::OpenTerminal);
    match effect {
        Effect::OpenTerminal { cwd } => {
            assert_eq!(cwd.canonicalize().unwrap(), dir.canonicalize().unwrap());
        }
        _ => panic!("expected the shell to be opened"),
    }
}

/// The pinned panel has no directory of its own, so there is nothing to open a
/// shell in.
#[test]
fn the_pinned_panel_has_no_shell() {
    let mut model = model_in(temp_dir());
    model.active_panel = ActivePanel::Pinned;

    let (_, effect) = update(model, Message::OpenTerminal);
    assert!(matches!(effect, Effect::None));
}

/// While the shell has the keys every one of them is its own, so lfm's own
/// bindings must not fire — `q` types a `q` rather than quitting.
#[test]
fn a_focused_shell_takes_every_key_but_the_one_that_leaves_it() {
    let quit = Event::Key(key(KeyCode::Char('q')));
    let msg = to_message(&quit, ActivePanel::LeftFiles, &InputMode::Terminal);
    assert!(matches!(msg, Some(Message::TerminalKey(_))));

    let leave = Event::Key(ctrl('o'));
    let msg = to_message(&leave, ActivePanel::LeftFiles, &InputMode::Terminal);
    assert!(matches!(msg, Some(Message::UnfocusTerminal)));
}

/// Leaving the shell hands the keys back without stopping it, so the file list
/// takes them again and `t` returns to a shell that kept its state.
#[test]
fn leaving_the_shell_keeps_it_running() {
    let mut model = model_in(temp_dir());
    model.terminal = Some(terminal::open(&temp_dir()).unwrap());
    assert!(matches!(input_mode(&model), InputMode::Terminal));

    let (model, _) = update(model, Message::UnfocusTerminal);
    assert!(
        model.terminal.is_some(),
        "the shell should still be running"
    );
    assert!(!matches!(input_mode(&model), InputMode::Terminal));

    let (model, effect) = update(model, Message::OpenTerminal);
    assert!(
        matches!(effect, Effect::None),
        "a second `t` must not start another shell"
    );
    assert!(matches!(input_mode(&model), InputMode::Terminal));
}

/// Closing the panel takes the shell down with it.
#[test]
fn closing_the_panel_ends_the_shell() {
    let mut model = model_in(temp_dir());
    model.terminal = Some(terminal::open(&temp_dir()).unwrap());

    let (model, _) = update(model, Message::Close(Surface::Terminal));
    assert!(model.terminal.is_none());
}

/// The shell exiting on its own — `exit`, Ctrl-D — closes the panel, rather than
/// leaving a dead screen at the bottom of the window.
#[test]
fn the_panel_goes_away_when_the_shell_exits() {
    let dir = temp_dir();
    let mut model = model_in(dir.clone());
    model.terminal = Some(terminal::open(&dir).unwrap());

    let panel = model.terminal.as_mut().unwrap();
    for c in "exit".chars() {
        panel.send_key(key(KeyCode::Char(c)));
    }
    panel.send_key(key(KeyCode::Enter));

    let deadline = Instant::now() + Duration::from_secs(10);
    while model.terminal.is_some() && Instant::now() < deadline {
        terminal::drain(&mut model);
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(model.terminal.is_none(), "the panel outlived its shell");
}

/// Rows of the rendered screen, top to bottom.
fn rendered_rows(model: &mut Model, width: u16, height: u16) -> Vec<String> {
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut term = ratatui::Terminal::new(backend).unwrap();
    term.draw(|frame| crate::view::view(model, frame)).unwrap();
    let buffer = term.backend().buffer().clone();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}

/// The shell panel lives across the bottom of the window, under the file list
/// rather than over it — the panels get shorter, nothing is covered up.
#[test]
fn the_shell_panel_is_drawn_at_the_bottom() {
    let dir = temp_dir();
    let mut model = model_in(dir.clone());
    model.terminal = Some(terminal::open(&dir).unwrap());

    let rows = rendered_rows(&mut model, 80, 24);
    let title = rows
        .iter()
        .position(|row| row.contains("shell —"))
        .expect("the shell panel should be on screen");
    assert!(
        title > rows.len() / 2,
        "the shell panel is at row {title}, not along the bottom"
    );
    // The file list keeps the space above it.
    assert!(
        rows[..title]
            .iter()
            .any(|row| row.contains(&dir.display().to_string()))
    );
    // And the hint bar says how to get back out of the shell.
    assert!(rows.last().unwrap().contains("Ctrl+O"));
}

/// The whole path end to end: a key typed at the panel reaches the shell, the
/// reader thread brings its output back, and the widget paints it.
#[test]
fn what_is_typed_at_the_panel_runs_and_is_shown() {
    let dir = temp_dir();
    let mut model = model_in(dir.clone());
    model.terminal = Some(terminal::open(&dir).unwrap());
    // Give the pty the size it will be drawn at before typing, so the echoed
    // command is not wrapped somewhere unexpected.
    rendered_rows(&mut model, 100, 30);

    let panel = model.terminal.as_mut().unwrap();
    for c in "echo lfm-panel-works".chars() {
        panel.send_key(key(KeyCode::Char(c)));
    }
    panel.send_key(key(KeyCode::Enter));

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut seen = false;
    while !seen && Instant::now() < deadline {
        terminal::drain(&mut model);
        // The command's own echo shows up first, so the run counts only once
        // the output line is there too.
        seen = rendered_rows(&mut model, 100, 30)
            .iter()
            .filter(|row| row.contains("lfm-panel-works"))
            .count()
            > 1;
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(seen, "the shell's output never reached the panel");
}

/// The pty is sized from the drawn panel, so the shell wraps its output where
/// the panel actually ends rather than at the width it was opened with.
#[test]
fn rendering_sizes_the_pty_to_the_panel() {
    let dir = temp_dir();
    let mut model = model_in(dir.clone());
    model.terminal = Some(terminal::open(&dir).unwrap());

    rendered_rows(&mut model, 60, 30);
    let (rows, cols) = model.terminal.as_ref().unwrap().size();
    // The panel gets 40% of the 29 rows left under the hint bar, and the pty
    // gets what is left of that inside the borders.
    assert_eq!(cols, 58);
    assert_eq!(rows, 10);
}

/// The bytes a shell expects for the keys that are not plain characters. Getting
/// these wrong is invisible until an arrow key prints garbage at the prompt.
#[test]
fn keys_are_encoded_the_way_a_terminal_sends_them() {
    assert_eq!(encode_key(key(KeyCode::Char('a'))).unwrap(), b"a");
    assert_eq!(encode_key(key(KeyCode::Enter)).unwrap(), b"\r");
    assert_eq!(encode_key(key(KeyCode::Backspace)).unwrap(), &[0x7f]);
    assert_eq!(encode_key(key(KeyCode::Esc)).unwrap(), &[0x1b]);
    assert_eq!(encode_key(key(KeyCode::Up)).unwrap(), b"\x1b[A");
    assert_eq!(encode_key(key(KeyCode::PageDown)).unwrap(), b"\x1b[6~");
    assert_eq!(encode_key(key(KeyCode::F(1))).unwrap(), b"\x1bOP");
    assert_eq!(encode_key(key(KeyCode::F(12))).unwrap(), b"\x1b[24~");
}

/// Ctrl-C has to reach the child as the interrupt byte, or a runaway command in
/// the panel could not be stopped.
#[test]
fn ctrl_keys_become_control_bytes() {
    assert_eq!(encode_key(ctrl('c')).unwrap(), &[0x03]);
    assert_eq!(encode_key(ctrl('d')).unwrap(), &[0x04]);
    // The shifted spelling is the same key, so it must encode the same.
    assert_eq!(encode_key(ctrl('C')).unwrap(), &[0x03]);
}

/// Alt is sent as an escape in front of the key's own bytes.
#[test]
fn alt_prefixes_an_escape() {
    let alt_b = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT);
    assert_eq!(encode_key(alt_b).unwrap(), b"\x1bb");
}

/// A bare modifier has nothing to send; forwarding it as anything would type
/// stray characters at the prompt.
#[test]
fn keys_with_no_encoding_are_dropped() {
    use ratatui::crossterm::event::ModifierKeyCode;
    assert!(encode_key(key(KeyCode::Modifier(ModifierKeyCode::LeftShift))).is_none());
    assert!(encode_key(key(KeyCode::F(20))).is_none());
}
