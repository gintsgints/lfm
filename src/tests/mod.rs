mod capture;
mod capture_view;
mod close;
mod copy_destination;
mod engine;
mod file_find;
mod file_mask;
mod file_view;
mod filter;
mod help_panel;
mod icons;
mod keys;
mod nav;
mod path_input;
mod pending_overwrite;
mod search;
mod search_panel;
mod theme;
mod transfer;
mod transfer_selection;
use std::{path::PathBuf, sync::mpsc, time::Duration};

use crate::engine::{EngineMsg, Kind, SearchEngine};
use crate::file_find::FileFindResult;
use crate::search::SearchResult;

/// The crate's own `src` directory: a real tree for the engine to index.
fn src_dir() -> PathBuf {
    root_dir().join("src")
}

/// The crate root, indexed when a test needs more than one kind of file.
fn root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Block until the worker reports a batch.
fn next_batch(engine: &SearchEngine) -> EngineMsg {
    for _ in 0..600 {
        match engine.try_recv() {
            Ok(msg) => return msg,
            Err(mpsc::TryRecvError::Empty) => std::thread::sleep(Duration::from_millis(100)),
            Err(mpsc::TryRecvError::Disconnected) => panic!("search worker died"),
        }
    }
    panic!("timed out waiting for results");
}

/// Block until the content-search batch for the newest query arrives.
fn wait_for_content(engine: &SearchEngine) -> Vec<SearchResult> {
    match next_batch(engine) {
        EngineMsg::Content {
            generation,
            results,
        } => {
            assert_eq!(
                generation,
                engine.generation(Kind::Content),
                "expected the batch for the newest query"
            );
            results
        }
        EngineMsg::Files { .. } => panic!("expected a content batch, got a file-find one"),
        EngineMsg::Failed(err) => panic!("indexing failed: {err}"),
    }
}

/// Block until the file-find batch for the newest query arrives.
fn wait_for_files(engine: &SearchEngine) -> Vec<FileFindResult> {
    match next_batch(engine) {
        EngineMsg::Files {
            generation,
            results,
        } => {
            assert_eq!(
                generation,
                engine.generation(Kind::Files),
                "expected the batch for the newest query"
            );
            results
        }
        EngineMsg::Content { .. } => panic!("expected a file-find batch, got a content one"),
        EngineMsg::Failed(err) => panic!("indexing failed: {err}"),
    }
}
