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
use crate::file_mask::FileMask;
/// Maximum number of fuzzy matches returned for a single query.
const MAX_RESULTS: usize = 200;

/// Upper bound on pages walked while filling a masked batch, so a mask that
/// admits almost nothing cannot walk the whole index.
const MAX_PAGES: usize = 20;

pub struct FileFindResult {
    pub path: PathBuf,
    pub rel_path: PathBuf,
}

/// Rank the indexed file names against `query` and return the top matches that
/// `mask` admits.
///
/// Ranking is a pass over the in-memory index, so there is nothing worth
/// aborting part-way through.
pub fn fuzzy_find(
    picker: &FilePicker,
    root: &Path,
    query: &str,
    mask: &FileMask,
    _abort: &Arc<AtomicBool>,
) -> Vec<FileFindResult> {
    #[cfg(feature = "debug")]
    let start = Instant::now();

    let parser = QueryParser::default();
    let parsed = parser.parse(query);
    let mut hits: Vec<FileFindResult> = Vec::new();
    let mut offset = 0;

    #[cfg(feature = "debug")]
    let mut total_matched = 0;

    for _ in 0..MAX_PAGES {
        let result = picker.fuzzy_search(
            &parsed,
            None,
            FuzzySearchOptions {
                max_threads: 0,
                pagination: PaginationArgs {
                    offset,
                    limit: MAX_RESULTS,
                },
                ..Default::default()
            },
        );

        #[cfg(feature = "debug")]
        {
            total_matched = result.total_matched;
        }

        hits.extend(result.items.iter().filter_map(|item| {
            let rel_path = PathBuf::from(item.relative_path(picker));
            mask.matches(&rel_path).then(|| FileFindResult {
                path: root.join(&rel_path),
                rel_path,
            })
        }));

        // Without a mask the first page is the answer, exactly as before; a
        // short page means the ranking is exhausted.
        if mask.is_empty()
            || hits.len() >= MAX_RESULTS
            || result.items.len() < MAX_RESULTS
            || offset + MAX_RESULTS >= result.total_matched
        {
            break;
        }
        offset += MAX_RESULTS;
    }

    hits.truncate(MAX_RESULTS);

    #[cfg(feature = "debug")]
    debug_log!(
        "file find {:?} for {query:?}: {} shown of {total_matched} match(es), {:.3}ms",
        root,
        hits.len(),
        start.elapsed().as_secs_f64() * 1000.0,
    );

    hits
}
