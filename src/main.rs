use std::{
    io,
    path::{Path, PathBuf},
    process::Stdio,
    sync::mpsc,
    time::{Duration, Instant},
};

use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event},
};
use ratatui_image::picker::Picker;

mod archive;
mod capture;
pub mod debug;
mod engine;
mod file_find;
mod file_mask;
mod icons;
mod image_view;
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
use model::{CaptureView, Model, ResultPanel};
use presets::OutputMode;
use update::{Effect, RunSpec, update};
use view::view;

/// How long the browsed directory must stay put before its index is built. A
/// filesystem walk cannot be cancelled once started, so stepping through
/// directories must not start one per directory.
const INDEX_DEBOUNCE: Duration = Duration::from_millis(300);

/// Poll interval while background work is running but has produced nothing yet.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

fn main() -> io::Result<()> {
    let choosedir = std::env::var_os("LFM_CHOOSEDIR").map(PathBuf::from);
    theme::init(theme::load());
    let terminal = ratatui::init();
    // Query the terminal's graphics support and font size before any other
    // escape sequence is exchanged with it: the query writes to stdio and reads
    // the answer back. A terminal that does not answer leaves the viewer
    // without images rather than failing.
    let picker = Picker::from_query_stdio().ok();
    let extended_keys = enable_extended_key_reporting();
    let result = run(terminal, extended_keys, picker);
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
/// (capture view, error modal, refreshed panels).
fn run_preset_command(terminal: &mut DefaultTerminal, mut model: Model, spec: RunSpec) -> Model {
    match spec.mode {
        OutputMode::Background => {
            let mut cmd = build_command(&spec);
            // Detach every stream: a background child sharing our terminal
            // would draw its output over the TUI.
            cmd.stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            if let Err(e) = cmd.spawn() {
                model.error_message = Some(format!("spawn '{}' failed: {e}", spec.label));
            }
            model
        }
        OutputMode::Capture => {
            let mut cmd = build_command(&spec);
            // The terminal is in raw mode and owned by the TUI: a child reading
            // stdin would swallow our keys and hang the UI.
            cmd.stdin(Stdio::null());
            match cmd.output() {
                Ok(out) => {
                    let mut buf = String::new();
                    buf.push_str(&String::from_utf8_lossy(&out.stdout));
                    if !out.stderr.is_empty() {
                        buf.push_str(&String::from_utf8_lossy(&out.stderr));
                    }
                    model.capture_view = Some(CaptureView {
                        label: spec.label,
                        exit_code: out.status.code(),
                        output: capture::sanitize(&buf),
                        scroll: 0,
                        viewport_width: 0,
                        viewport_height: 0,
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

fn run(
    mut terminal: DefaultTerminal,
    extended_keys: bool,
    picker: Option<Picker>,
) -> io::Result<PathBuf> {
    let mut model = Model::init(state::load())?;
    model.picker = picker;
    let mut progress_rx: Option<mpsc::Receiver<transfer::ProgressMsg>> = None;
    let mut index = Index::default();

    loop {
        terminal.draw(|frame| view(&mut model, frame))?;

        // A finished transfer means the tree changed under the index.
        let was_transferring = progress_rx.is_some();
        let (m, got_progress) = drain_progress(model, &mut progress_rx);
        model = m;
        index.stale |= was_transferring && progress_rx.is_none();

        let got_results = drain_index(&mut model, &mut index);

        // Pick up whatever the open image's worker has decoded or re-encoded.
        let got_image = image_view::drain(&mut model);

        // Start pre-warming the directory being browsed once it settles. Not
        // while a panel is open — replacing the index under an open panel would
        // strand the query it is waiting on — and not during a transfer, whose
        // writes would make the fresh index stale anyway.
        if model.content_search.is_none() && model.file_find.is_none() && progress_rx.is_none() {
            index.schedule(model.active_dir());
            index.tick();
        }

        // A panel is outstanding while the index is still building or its query
        // has not reported back yet. The engine itself is long-lived, so it must
        // not keep the loop spinning once both panels are idle.
        let search_pending =
            pending(model.content_search.as_ref()) || pending(model.file_find.as_ref());

        // The image worker answers on its own channel, so the loop polls while
        // it owes an answer instead of blocking on the keyboard.
        let image_pending = image_view::pending(&model);

        let timeout = if got_progress || got_results || got_image {
            Some(Duration::ZERO)
        } else if progress_rx.is_some() || search_pending || image_pending {
            Some(POLL_INTERVAL)
        } else {
            index.debounce_remaining()
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
            index.stale |= msg.mutates_filesystem();
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
                    sync_indexing(model.content_search.as_mut(), index.of(root));
                }
                Effect::StartContentSearch { root, query, mask } => {
                    let engine = index.of(root);
                    engine.search(Kind::Content, query, mask);
                    sync_indexing(model.content_search.as_mut(), engine);
                }
                Effect::RunCommand { spec } => {
                    // The command may write anywhere, so assume it did.
                    index.stale = true;
                    model = run_preset_command(&mut terminal, model, spec);
                }
                Effect::PrepareFileFind { root } => {
                    sync_indexing(model.file_find.as_mut(), index.of(root));
                }
                Effect::StartFileFind { root, query, mask } => {
                    let engine = index.of(root);
                    engine.search(Kind::Files, query, mask);
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

/// The search index both query panels run against, plus the policy for when it
/// gets built.
///
/// One index serves the content search and the file find, and it is built once
/// per directory: browsing into a directory pre-warms it, so a panel opened
/// there usually has results waiting instead of a directory walk.
#[derive(Default)]
struct Index {
    engine: Option<SearchEngine>,
    /// lfm wrote to the filesystem since the index was built, so a search would
    /// answer from a stale picture of the tree. Cleared by the next rebuild.
    stale: bool,
    /// Directory queued for pre-warming, and when its debounce expires. A walk
    /// cannot be cancelled once started, so stepping through directories must
    /// not start one per directory.
    pending: Option<(PathBuf, Instant)>,
}

impl Index {
    /// Queue `dir` for pre-warming unless it is already indexed or queued.
    fn schedule(&mut self, dir: &Path) {
        if self.engine.as_ref().is_some_and(|e| e.root() == dir) {
            self.pending = None;
            return;
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|(queued, _)| queued == dir)
        {
            return; // already waiting on this directory — don't push the deadline out
        }
        self.pending = Some((dir.to_path_buf(), Instant::now() + INDEX_DEBOUNCE));
    }

    /// Build the queued index once its directory has stayed put long enough.
    fn tick(&mut self) {
        if self
            .pending
            .as_ref()
            .is_some_and(|(_, at)| Instant::now() >= *at)
        {
            let (dir, _) = self.pending.take().expect("pending was just checked");
            self.engine = Some(SearchEngine::spawn(dir));
            self.stale = false;
        }
    }

    /// How long until the queued index starts, for the event poll.
    fn debounce_remaining(&self) -> Option<Duration> {
        self.pending
            .as_ref()
            .map(|(_, at)| at.saturating_duration_since(Instant::now()))
    }

    /// Return the engine for `root`, rebuilding when the pre-warmed index is for
    /// a different directory or went stale under a filesystem change.
    fn of(&mut self, root: PathBuf) -> &mut SearchEngine {
        if self.stale || self.engine.as_ref().is_none_or(|e| e.root() != root) {
            self.engine = Some(SearchEngine::spawn(root));
            self.stale = false;
        }
        self.pending = None;
        self.engine.as_mut().expect("engine was just ensured")
    }
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
fn drain_index(model: &mut Model, index: &mut Index) -> bool {
    if index.engine.is_none() {
        return false;
    }
    if model.content_search.is_none() && model.file_find.is_none() {
        // Both panels closed: stop the in-flight query and throw away whatever
        // it already produced, but keep the index for the next open.
        if let Some(engine) = index.engine.as_ref() {
            engine.abort_current();
            while engine.try_recv().is_ok() {}
        }
        return false;
    }

    let mut changed = false;
    loop {
        // Read everything needed off the engine up front, so the arms below are
        // free to drop it.
        let (content_gen, files_gen, indexing, msg) = match index.engine.as_ref() {
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
                index.engine = None;
                changed = true;
                break;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                finish(model);
                index.engine = None;
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
