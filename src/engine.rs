#[cfg(feature = "debug")]
use std::time::Instant;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

use fff_search::file_picker::{FilePicker, FilePickerOptions};

#[cfg(feature = "debug")]
use crate::debug_log;
use crate::file_find::{self, FileFindResult};
use crate::file_mask::FileMask;
use crate::search::{self, SearchResult};

/// Which panel a query belongs to. Both are served by the same index.
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub enum Kind {
    /// Content search: grep the indexed files.
    Content,
    /// File find: fuzzy-match the indexed file names.
    Files,
}

/// What the worker reports back for a query.
pub enum EngineMsg {
    /// The complete result set for one query of the matching kind. `generation`
    /// says which query produced it, so a batch for an already-superseded query
    /// can be dropped.
    Content {
        generation: u64,
        results: Vec<SearchResult>,
    },
    Files {
        generation: u64,
        results: Vec<FileFindResult>,
    },
    /// Indexing failed — this engine cannot answer queries.
    Failed(String),
}

/// A query handed to the worker.
struct Request {
    kind: Kind,
    generation: u64,
    text: String,
    /// Glob patterns limiting which files the query looks at; empty means all.
    mask: String,
}

/// Handle to the background search worker.
///
/// The worker indexes `root` **once** with fff-search and answers every
/// subsequent query from that in-memory index, so a keystroke costs one search
/// pass rather than a fresh directory walk. Both panels share the one index:
/// `FilePicker` is not `Sync`, so a second engine would mean walking the same
/// tree twice.
pub struct SearchEngine {
    root: PathBuf,
    requests: mpsc::Sender<Request>,
    results: mpsc::Receiver<EngineMsg>,
    /// Set to stop the in-flight query; the worker clears it before each query.
    abort: Arc<AtomicBool>,
    /// Cleared once the index is built.
    indexing: Arc<AtomicBool>,
    content_generation: u64,
    files_generation: u64,
}

impl SearchEngine {
    /// Spawn a worker for `root` and start indexing immediately, so the index is
    /// usually ready by the time a query is typed.
    pub fn spawn(root: PathBuf) -> Self {
        let (req_tx, req_rx) = mpsc::channel();
        let (res_tx, res_rx) = mpsc::channel();
        let abort = Arc::new(AtomicBool::new(false));
        let indexing = Arc::new(AtomicBool::new(true));
        {
            let root = root.clone();
            let abort = Arc::clone(&abort);
            let indexing = Arc::clone(&indexing);
            std::thread::spawn(move || worker(&root, &req_rx, &res_tx, &abort, &indexing));
        }
        Self {
            root,
            requests: req_tx,
            results: res_rx,
            abort,
            indexing,
            content_generation: 0,
            files_generation: 0,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn is_indexing(&self) -> bool {
        self.indexing.load(Ordering::Relaxed)
    }

    /// Generation of the most recently submitted query of `kind`.
    pub fn generation(&self, kind: Kind) -> u64 {
        match kind {
            Kind::Content => self.content_generation,
            Kind::Files => self.files_generation,
        }
    }

    /// Cancel the in-flight query and queue `text`, restricted to the files
    /// `mask` admits, as the newest one.
    pub fn search(&mut self, kind: Kind, text: String, mask: String) {
        let generation = match kind {
            Kind::Content => {
                self.content_generation += 1;
                self.content_generation
            }
            Kind::Files => {
                self.files_generation += 1;
                self.files_generation
            }
        };
        self.abort.store(true, Ordering::Relaxed);
        let _ = self.requests.send(Request {
            kind,
            generation,
            text,
            mask,
        });
    }

    /// Stop the in-flight query without queueing anything — used when the panel
    /// closes, which keeps the index around for the next open.
    pub fn abort_current(&self) {
        self.abort.store(true, Ordering::Relaxed);
    }

    pub fn try_recv(&self) -> Result<EngineMsg, mpsc::TryRecvError> {
        self.results.try_recv()
    }
}

impl Drop for SearchEngine {
    fn drop(&mut self) {
        // Unblock an in-flight query so the worker notices the closed request
        // channel and exits promptly.
        self.abort.store(true, Ordering::Relaxed);
    }
}

fn worker(
    root: &Path,
    requests: &mpsc::Receiver<Request>,
    results: &mpsc::Sender<EngineMsg>,
    abort: &Arc<AtomicBool>,
    indexing: &AtomicBool,
) {
    let index = build_index(root);
    indexing.store(false, Ordering::Relaxed);
    let picker = match index {
        Ok(picker) => picker,
        Err(err) => {
            let _ = results.send(EngineMsg::Failed(err));
            return;
        }
    };

    while let Ok(request) = requests.recv() {
        // Several characters may have been typed while the previous query ran;
        // only the newest one still matters. Only one panel is open at a time,
        // so a dropped request of the other kind is one nobody is waiting for.
        let request = newest(requests, request);
        abort.store(false, Ordering::Relaxed);
        // Compiled once per query, here rather than on the UI thread.
        let mask = FileMask::parse(&request.mask);
        let batch = match request.kind {
            Kind::Content => EngineMsg::Content {
                generation: request.generation,
                results: search::grep(&picker, root, &request.text, &mask, abort),
            },
            Kind::Files => EngineMsg::Files {
                generation: request.generation,
                results: file_find::fuzzy_find(&picker, root, &request.text, &mask, abort),
            },
        };
        if results.send(batch).is_err() {
            return; // receiver dropped — engine discarded
        }
    }
}

/// Collapse a backlog of queued queries down to the most recent one.
fn newest(requests: &mpsc::Receiver<Request>, mut request: Request) -> Request {
    while let Ok(next) = requests.try_recv() {
        request = next;
    }
    request
}

fn build_index(root: &Path) -> Result<FilePicker, String> {
    #[cfg(feature = "debug")]
    let start = Instant::now();

    let options = FilePickerOptions {
        base_path: root.to_string_lossy().into_owned(),
        watch: false,
        // A file manager routinely browses the home directory, so allow
        // indexing it. The filesystem root is left off by default — walking it
        // would be enormous.
        enable_home_dir_scanning: true,
        ..Default::default()
    };

    let mut picker = FilePicker::new(options).map_err(|e| format!("cannot index: {e}"))?;
    picker
        .collect_files()
        .map_err(|e| format!("cannot index: {e}"))?;

    #[cfg(feature = "debug")]
    debug_log!(
        "indexed {:?}: {} file(s), {:.3}ms",
        root,
        picker.get_files().len(),
        start.elapsed().as_secs_f64() * 1000.0,
    );

    Ok(picker)
}
