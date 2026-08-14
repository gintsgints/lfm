#[cfg(feature = "debug")]
use std::time::Instant;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
};

use fff_search::file_picker::{FilePicker, FuzzySearchOptions};
use fff_search::{PaginationArgs, QueryParser};

#[cfg(feature = "debug")]
use crate::debug_log;
/// Maximum number of fuzzy matches returned for a single query.
const MAX_RESULTS: usize = 200;

pub struct FileFindResult {
    pub path: PathBuf,
    pub rel_path: PathBuf,
}

/// Rank the indexed file names against `query` and return the top matches.
///
/// Ranking is a pass over the in-memory index, so there is nothing worth
/// aborting part-way through.
pub fn fuzzy_find(
    picker: &FilePicker,
    root: &Path,
    query: &str,
    _abort: &Arc<AtomicBool>,
) -> Vec<FileFindResult> {
    #[cfg(feature = "debug")]
    let start = Instant::now();

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
    let total_matched = result.total_matched;

    let hits: Vec<FileFindResult> = result
        .items
        .iter()
        .map(|item| {
            let rel_path = PathBuf::from(item.relative_path(picker));
            FileFindResult {
                path: root.join(&rel_path),
                rel_path,
            }
        })
        .collect();

    #[cfg(feature = "debug")]
    debug_log!(
        "file find {:?} for {query:?}: {} shown of {total_matched} match(es), {:.3}ms",
        root,
        hits.len(),
        start.elapsed().as_secs_f64() * 1000.0,
    );

    hits
}
