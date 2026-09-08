//! The embedded terminal panel: a real shell running on a pty, drawn at the
//! bottom of the screen by `tui-term`.
//!
//! The shell is started in whatever directory the active file panel is showing,
//! so `t` gives a prompt where the user is already looking. Its output is read
//! by a worker thread and handed to the UI thread as raw bytes, which are fed to
//! a [`vt100::Parser`]; the widget then draws that parser's screen. Nothing
//! blocks the redraw loop: a read that has not happened yet simply leaves the
//! screen as it was.
//!
//! The panel owns the child. Dropping it — closing the panel, or quitting lfm —
//! kills the shell, and the reader thread stops when its channel end goes with
//! it. The shell exiting on its own (`exit`, Ctrl-D) closes the reader's end
//! instead, which is what tells the UI thread to take the panel down.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::model::Model;

/// Size the pty is opened at, before the first render reports the real one. A
/// shell started at 0x0 would wrap its prompt at the first column, so the panel
/// opens at an ordinary size and resizes once it knows the area it got.
const INITIAL_ROWS: u16 = 24;
const INITIAL_COLS: u16 = 80;

/// How much of the child's output is taken per read.
const READ_CHUNK: usize = 8192;

/// A shell running on a pty, and the parser turning its output into a screen.
pub struct TerminalPanel {
    parser: vt100::Parser,
    /// Raw output from the reader thread. A disconnect means the shell exited.
    rx: Receiver<Vec<u8>>,
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    /// Directory the shell was started in, shown in the panel title.
    pub cwd: PathBuf,
    /// Whether keys go to the shell rather than to the file list. `t` and
    /// `Ctrl+O` flip it; the panel stays open either way.
    pub focused: bool,
    rows: u16,
    cols: u16,
}

impl TerminalPanel {
    /// The screen the widget draws.
    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    /// The size the pty is currently running at, as `(rows, cols)`.
    #[cfg(test)]
    pub fn size(&self) -> (u16, u16) {
        (self.rows, self.cols)
    }

    /// Match the pty and the parser to the panel's drawn size. Called from the
    /// renderer, which is the only place the real size is known; a no-op unless
    /// the size actually changed, so it does not signal the child on every
    /// frame.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        let (rows, cols) = (rows.max(1), cols.max(1));
        if rows == self.rows && cols == self.cols {
            return;
        }
        self.rows = rows;
        self.cols = cols;
        self.parser.screen_mut().set_size(rows, cols);
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }

    /// Forward one keystroke to the shell. Keys with no terminal encoding are
    /// dropped rather than sent as something else.
    pub fn send_key(&mut self, key: KeyEvent) {
        let Some(bytes) = encode_key(key) else {
            return;
        };
        // A write that fails means the child is gone; the reader thread notices
        // the same thing and closes the panel.
        let _ = self.writer.write_all(&bytes);
        let _ = self.writer.flush();
    }
}

impl Drop for TerminalPanel {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Start a shell in `cwd` and return the panel drawing it.
pub fn open(cwd: &Path) -> Result<TerminalPanel, String> {
    let pty = native_pty_system();
    let pair = pty
        .openpty(PtySize {
            rows: INITIAL_ROWS,
            cols: INITIAL_COLS,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("open pty failed: {e}"))?;

    let mut cmd = CommandBuilder::new(shell());
    cmd.cwd(cwd);
    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("start shell failed: {e}"))?;
    // The child holds its own end of the pty now. Ours has to go, or the reader
    // below never sees EOF when the shell exits.
    drop(pair.slave);

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("read pty failed: {e}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("write pty failed: {e}"))?;

    let (tx, rx) = channel();
    std::thread::spawn(move || read_output(reader, &tx));

    Ok(TerminalPanel {
        parser: vt100::Parser::new(INITIAL_ROWS, INITIAL_COLS, 0),
        rx,
        writer,
        master: pair.master,
        child,
        cwd: cwd.to_path_buf(),
        focused: true,
        rows: INITIAL_ROWS,
        cols: INITIAL_COLS,
    })
}

/// The shell to run: the user's own, falling back to the platform default.
fn shell() -> std::ffi::OsString {
    #[cfg(windows)]
    let (var, fallback) = ("COMSPEC", "cmd.exe");
    #[cfg(not(windows))]
    let (var, fallback) = ("SHELL", "/bin/sh");
    std::env::var_os(var).unwrap_or_else(|| fallback.into())
}

/// Hand the child's output to the UI thread until either end goes away.
fn read_output(mut reader: Box<dyn Read + Send>, out: &Sender<Vec<u8>>) {
    let mut buf = [0u8; READ_CHUNK];
    loop {
        // A read error is the pty closing under us, which means the same thing
        // as EOF here.
        let Ok(read) = reader.read(&mut buf) else {
            return;
        };
        if read == 0 || out.send(buf[..read].to_vec()).is_err() {
            return;
        }
    }
}

/// Feed the UI thread whatever the shell has written. Returns whether the panel
/// changed and should be redrawn.
///
/// A closed channel means the shell exited, so the panel goes away and the file
/// panels are reloaded — the shell may well have been used to change them.
pub fn drain(model: &mut Model) -> bool {
    let Some(panel) = model.terminal.as_mut() else {
        return false;
    };
    let mut changed = false;
    let mut exited = false;
    loop {
        match panel.rx.try_recv() {
            Ok(bytes) => {
                panel.parser.process(&bytes);
                changed = true;
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                exited = true;
                break;
            }
        }
    }
    if exited {
        close(model);
        return true;
    }
    changed
}

/// Take the panel down and reload the file panels, which the shell may have
/// written to.
pub fn close(model: &mut Model) {
    model.terminal = None;
    let left = model.left_files.current_dir.clone();
    model.left_files.navigate_to(left);
    let right = model.right_files.current_dir.clone();
    model.right_files.navigate_to(right);
}

/// Whether a terminal is open, which keeps the event loop polling for its
/// output instead of blocking on the keyboard.
pub fn pending(model: &Model) -> bool {
    model.terminal.is_some()
}

/// Whether an open shell currently has the keys.
pub fn is_focused(model: &Model) -> bool {
    model.terminal.as_ref().is_some_and(|t| t.focused)
}

/// The bytes a terminal application would receive for `key`, or `None` for a key
/// that has no encoding (bare modifiers, media keys).
pub fn encode_key(key: KeyEvent) -> Option<Vec<u8>> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    let mut bytes = match key.code {
        KeyCode::Char(c) if ctrl => vec![control_byte(c)?],
        KeyCode::Char(c) => c.to_string().into_bytes(),
        // The pty's line discipline turns the carriage return into whatever the
        // shell expects, so both platforms send the same thing.
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::F(n) => function_key(n)?,
        _ => return None,
    };

    // Alt is sent the way every terminal sends it: the key's own bytes, with an
    // escape in front.
    if alt {
        bytes.insert(0, 0x1b);
    }
    Some(bytes)
}

/// The control character `Ctrl` produces with `c`, following the usual ASCII
/// rule of clearing the top three bits.
fn control_byte(c: char) -> Option<u8> {
    let byte = match c {
        'a'..='z' => (c as u8) - b'a' + 1,
        'A'..='Z' => (c as u8) - b'A' + 1,
        ' ' | '@' => 0,
        '[' => 27,
        '\\' => 28,
        ']' => 29,
        '^' => 30,
        '_' | '?' => 31,
        _ => return None,
    };
    Some(byte)
}

/// The escape sequence for a function key. F1-F4 use the older SS3 form that
/// terminals still send; the rest use the numbered CSI form.
fn function_key(n: u8) -> Option<Vec<u8>> {
    let bytes = match n {
        1 => b"\x1bOP".to_vec(),
        2 => b"\x1bOQ".to_vec(),
        3 => b"\x1bOR".to_vec(),
        4 => b"\x1bOS".to_vec(),
        5 => b"\x1b[15~".to_vec(),
        6 => b"\x1b[17~".to_vec(),
        7 => b"\x1b[18~".to_vec(),
        8 => b"\x1b[19~".to_vec(),
        9 => b"\x1b[20~".to_vec(),
        10 => b"\x1b[21~".to_vec(),
        11 => b"\x1b[23~".to_vec(),
        12 => b"\x1b[24~".to_vec(),
        _ => return None,
    };
    Some(bytes)
}
