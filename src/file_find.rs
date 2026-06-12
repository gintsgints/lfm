#[cfg(feature = "debug")]
use std::time::Instant;
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

use fff_search::file_picker::{FilePicker, FilePickerOptions, FuzzySearchOptions};
use fff_search::{PaginationArgs, QueryParser};

#[cfg(feature = "debug")]
use crate::debug_log;

/// Maximum number of fuzzy matches returned for a single query.
const MAX_RESULTS: usize = 200;

pub struct FileFindResult {
    pub path: PathBuf,
    pub rel_path: PathBuf,
}

pub enum FileFindMsg {
    Hit(FileFindResult),
    Done,
}

/// Fuzzy-find files by name under `root` using the fff-search engine.
///
/// Walks the tree synchronously (no background watcher or frecency database),
/// ranks names against `query`, and streams the top matches back over `tx`.
pub fn run_file_find(root: &Path, query: &str, tx: &mpsc::Sender<FileFindMsg>) {
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

    let Ok(mut picker) = FilePicker::new(options) else {
        let _ = tx.send(FileFindMsg::Done);
        return;
    };
    if picker.collect_files().is_err() {
        let _ = tx.send(FileFindMsg::Done);
        return;
    }

    let parser = QueryParser::default();
    let parsed = parser.parse(query);
    let result = picker.fuzzy_search(
        &parsed,
        None,
        FuzzySearchOptions {
            max_threads: 0,
            pagination: PaginationArgs {
                offset: 0,
                limit: MAX_RESULTS,
            },
            ..Default::default()
        },
    );

    #[cfg(feature = "debug")]
    let returned = result.items.len();

    for item in result.items {
        let rel_path = PathBuf::from(item.relative_path(&picker));
        let hit = FileFindMsg::Hit(FileFindResult {
            path: root.join(&rel_path),
            rel_path,
        });
        if tx.send(hit).is_err() {
            return; // receiver dropped — search cancelled
        }
    }

    #[cfg(feature = "debug")]
    debug_log!(
        "file find {:?} for {query:?}: {returned} shown of {} match(es), {:.3}ms",
        root,
        result.total_matched,
        start.elapsed().as_secs_f64() * 1000.0,
    );

    let _ = tx.send(FileFindMsg::Done);
}
