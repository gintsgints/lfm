use std::{io, path::PathBuf, sync::mpsc, time::Duration};

use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event},
};

mod archive;
pub mod debug;
mod engine;
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

use engine::{EngineMsg, Kind, SearchEngine};
use keys::{
    disable_extended_key_reporting, enable_extended_key_reporting, input_mode, normalize_key_event,
    to_message,
};
use message::Message;
use model::{CapturePopup, Model, ResultPanel};
use presets::OutputMode;
use update::{Effect, RunSpec, update};
use view::view;

/// Poll interval while background work is running but has produced nothing yet.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

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

fn open_with_default_app(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", "", &path.to_string_lossy()])
        .spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

fn run(mut terminal: DefaultTerminal, extended_keys: bool) -> io::Result<PathBuf> {
    let mut model = Model::init(state::load())?;
    let mut progress_rx: Option<mpsc::Receiver<transfer::ProgressMsg>> = None;
    // Kept across panel sessions so a root is indexed once, not once per query.
    let mut search_engine: Option<SearchEngine> = None;

    loop {
        terminal.draw(|frame| view(&model, frame))?;

        let (m, got_progress) = drain_progress(model, &mut progress_rx);
        model = m;
        let got_results = drain_index(&mut model, &mut search_engine);

        // A panel is outstanding while the index is still building or its query
        // has not reported back yet. The engine itself is long-lived, so it must
        // not keep the loop spinning once both panels are idle.
        let search_pending =
            pending(model.content_search.as_ref()) || pending(model.file_find.as_ref());

        let timeout = if got_progress || got_results {
            Some(Duration::ZERO)
        } else if progress_rx.is_some() || search_pending {
            Some(POLL_INTERVAL)
        } else {
            None
        };
        let Some(event) = next_event(timeout)? else {
            continue;
        };

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
                Effect::OpenDefault(path) => open_with_default_app(&path),
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
                Effect::PrepareContentSearch { root } => {
                    sync_indexing(
                        model.content_search.as_mut(),
                        ensure_engine(&mut search_engine, root),
                    );
                }
                Effect::StartContentSearch { root, query } => {
                    let engine = ensure_engine(&mut search_engine, root);
                    engine.search(Kind::Content, query);
                    sync_indexing(model.content_search.as_mut(), engine);
                }
                Effect::RunCommand { spec } => {
                    model = run_preset_command(&mut terminal, model, spec);
                }
                Effect::PrepareFileFind { root } => {
                    sync_indexing(
                        model.file_find.as_mut(),
                        ensure_engine(&mut search_engine, root),
                    );
                }
                Effect::StartFileFind { root, query } => {
                    let engine = ensure_engine(&mut search_engine, root);
                    engine.search(Kind::Files, query);
                    sync_indexing(model.file_find.as_mut(), engine);
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

/// Decide how long to wait for input.
///
/// Right after draining progress/search results, redraw promptly with a
/// zero-timeout poll — but still service the keyboard so a fast-producing
/// background thread (e.g. a search matching many lines) can't starve input and
/// freeze the UI. `None` blocks until the next event.
fn next_event(timeout: Option<Duration>) -> io::Result<Option<Event>> {
    let Some(timeout) = timeout else {
        return Ok(Some(event::read()?));
    };
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

/// Return the engine for `root`, spawning one (and dropping an engine indexing a
/// different root) only when needed. Reusing the engine is what keeps a tree
/// indexed once per root instead of once per query.
fn ensure_engine(engine: &mut Option<SearchEngine>, root: PathBuf) -> &mut SearchEngine {
    if engine.as_ref().is_none_or(|e| e.root() != root) {
        *engine = Some(SearchEngine::spawn(root));
    }
    engine.as_mut().expect("engine was just ensured")
}

/// Mirror the engine's indexing state onto the panel waiting on it.
fn sync_indexing<T>(panel: Option<&mut ResultPanel<T>>, engine: &SearchEngine) {
    if let Some(panel) = panel {
        panel.indexing = engine.is_indexing();
    }
}

/// Whether `panel` is still waiting on the engine.
fn pending<T>(panel: Option<&ResultPanel<T>>) -> bool {
    panel.is_some_and(|p| p.indexing || !p.done)
}

/// Move whatever the engine has produced into the panel that asked for it.
/// Returns whether a panel changed.
fn drain_index(model: &mut Model, engine: &mut Option<SearchEngine>) -> bool {
    if engine.is_none() {
        return false;
    }
    if model.content_search.is_none() && model.file_find.is_none() {
        // Both panels closed: stop the in-flight query and throw away whatever
        // it already produced, but keep the index for the next open.
        if let Some(engine) = engine.as_ref() {
            engine.abort_current();
            while engine.try_recv().is_ok() {}
        }
        return false;
    }

    let mut changed = false;
    loop {
        // Read everything needed off the engine up front, so the arms below are
        // free to drop it.
        let (content_gen, files_gen, indexing, msg) = match engine.as_ref() {
            None => break,
            Some(engine) => (
                engine.generation(Kind::Content),
                engine.generation(Kind::Files),
                engine.is_indexing(),
                engine.try_recv(),
            ),
        };
        set_indexing(model, indexing);
        match msg {
            Ok(EngineMsg::Content {
                generation,
                results,
            }) => {
                changed |= apply(&mut model.content_search, generation, content_gen, results);
            }
            Ok(EngineMsg::Files {
                generation,
                results,
            }) => {
                changed |= apply(&mut model.file_find, generation, files_gen, results);
            }
            // The worker is gone either way, so nothing is outstanding — leaving
            // a flag set would spin the event loop.
            Ok(EngineMsg::Failed(err)) => {
                finish(model);
                model.error_message = Some(err);
                *engine = None;
                changed = true;
                break;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                finish(model);
                *engine = None;
                changed = true;
                break;
            }
            Err(mpsc::TryRecvError::Empty) => break,
        }
    }
    changed
}

/// Replace a panel's results with a finished batch, ignoring a batch whose query
/// has already been superseded — the panel cleared its list for the newer one.
fn apply<T>(panel: &mut Option<ResultPanel<T>>, batch: u64, current: u64, results: Vec<T>) -> bool {
    if batch != current {
        return false;
    }
    let Some(panel) = panel.as_mut() else {
        return false;
    };
    panel.results = results;
    panel.selection = 0;
    panel.done = true;
    true
}

fn set_indexing(model: &mut Model, indexing: bool) {
    if let Some(panel) = model.content_search.as_mut() {
        panel.indexing = indexing;
    }
    if let Some(panel) = model.file_find.as_mut() {
        panel.indexing = indexing;
    }
}

/// Mark both panels as having nothing outstanding.
fn finish(model: &mut Model) {
    set_indexing(model, false);
    if let Some(panel) = model.content_search.as_mut() {
        panel.done = true;
    }
    if let Some(panel) = model.file_find.as_mut() {
        panel.done = true;
    }
}
