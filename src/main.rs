use std::{io, path::PathBuf, sync::mpsc, time::Duration};

use ratatui::{DefaultTerminal, crossterm::event};

mod archive;
pub mod debug;
mod file_find;
mod icons;
mod keys;
mod message;
mod model;
mod presets;
mod search;
mod state;
#[cfg(test)]
mod tests;
mod theme;
mod transfer;
mod ui;
mod update;
mod view;

use keys::{
    disable_extended_key_reporting, enable_extended_key_reporting, input_mode, normalize_key_event,
    to_message,
};
use message::Message;
use model::{CapturePopup, Model};
use presets::OutputMode;
use update::{Effect, RunSpec, update};
use view::view;

fn main() -> io::Result<()> {
    let choosedir = std::env::var_os("LFM_CHOOSEDIR").map(PathBuf::from);
    theme::init(theme::load());
    let terminal = ratatui::init();
    let extended_keys = enable_extended_key_reporting();
    let result = run(terminal, extended_keys);
    disable_extended_key_reporting();
    ratatui::restore();
    let dir = result?;
    if let Some(path) = choosedir {
        let _ = std::fs::write(path, dir.display().to_string());
    }
    Ok(())
}

fn build_command(spec: &RunSpec) -> std::process::Command {
    if let Some(argv) = &spec.argv {
        let mut iter = argv.iter();
        let program = iter.next().cloned().unwrap_or_default();
        let mut cmd = std::process::Command::new(program);
        cmd.args(iter);
        cmd.current_dir(&spec.cwd);
        cmd
    } else {
        let shell_cmd = spec.shell.clone().unwrap_or_default();
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(shell_cmd);
        cmd.current_dir(&spec.cwd);
        cmd
    }
}

/// Execute a preset command and update the model with any UI-visible result
/// (capture popup, error modal, refreshed panels).
fn run_preset_command(terminal: &mut DefaultTerminal, mut model: Model, spec: RunSpec) -> Model {
    match spec.mode {
        OutputMode::Background => {
            let mut cmd = build_command(&spec);
            if let Err(e) = cmd.spawn() {
                model.error_message = Some(format!("spawn '{}' failed: {e}", spec.label));
            }
            model
        }
        OutputMode::Capture => {
            let mut cmd = build_command(&spec);
            match cmd.output() {
                Ok(out) => {
                    let mut buf = String::new();
                    buf.push_str(&String::from_utf8_lossy(&out.stdout));
                    if !out.stderr.is_empty() {
                        buf.push_str(&String::from_utf8_lossy(&out.stderr));
                    }
                    model.capture_popup = Some(CapturePopup {
                        label: spec.label,
                        exit_code: out.status.code(),
                        output: buf,
                        scroll: 0,
                    });
                }
                Err(e) => {
                    model.error_message = Some(format!("spawn '{}' failed: {e}", spec.label));
                }
            }
            model
        }
        OutputMode::Block => {
            disable_extended_key_reporting();
            ratatui::restore();
            let mut cmd = build_command(&spec);
            let status = cmd.status();
            // Pause so the user can read the command's output before the TUI
            // wipes it. Done in cooked stdin mode (Enter to continue).
            println!();
            println!("[Press Enter to return to lfm]");
            let mut buf = String::new();
            let _ = std::io::stdin().read_line(&mut buf);
            *terminal = ratatui::init();
            enable_extended_key_reporting();
            if let Err(e) = status {
                model.error_message = Some(format!("spawn '{}' failed: {e}", spec.label));
            }
            // Refresh panels here so any filesystem changes are visible.
            let left = model.left_files.current_dir.clone();
            model.left_files.navigate_to(left);
            let right = model.right_files.current_dir.clone();
            model.right_files.navigate_to(right);
            model
        }
    }
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

fn run(mut terminal: DefaultTerminal, extended_keys: bool) -> io::Result<PathBuf> {
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
        keys::log_key(&event);

        let event = normalize_key_event(event, extended_keys);

        // Track Shift on its own so the hint bar can show the shifted command
        // set while it is held. Only fires when the terminal reports release
        // events (Kitty protocol); otherwise `shift_held` stays false.
        if let Some(held) = keys::shift_held_change(&event) {
            model = update(model, Message::SetShiftHeld(held)).0;
            continue;
        }

        // A key release must not re-trigger a command — the Kitty protocol
        // reports one for every press.
        if keys::is_key_release(&event) {
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
                Effect::RunCommand { spec } => {
                    model = run_preset_command(&mut terminal, model, spec);
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
