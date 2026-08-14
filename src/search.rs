#[cfg(feature = "debug")]
use std::time::Instant;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
};

use fff_search::file_picker::FilePicker;
use fff_search::grep::{GrepMode, GrepSearchOptions, parse_grep_query};

#[cfg(feature = "debug")]
use crate::debug_log;
/// Maximum number of matches collected for a single query.
const MAX_RESULTS: usize = 200;

/// Per-query wall-clock budget. A pathological pattern returns partial results
/// instead of stalling the panel.
const TIME_BUDGET_MS: u64 = 2_000;

pub struct SearchResult {
    pub path: PathBuf,
    pub rel_path: PathBuf,
    pub line_number: usize,
    pub line: String,
}

/// Grep the indexed files for `query` as literal text.
pub fn grep(
    picker: &FilePicker,
    root: &Path,
    query: &str,
    abort: &Arc<AtomicBool>,
) -> Vec<SearchResult> {
    #[cfg(feature = "debug")]
    let start = Instant::now();

    let parsed = parse_grep_query(query);
    let options = GrepSearchOptions {
        page_limit: MAX_RESULTS,
        mode: GrepMode::PlainText,
        time_budget_ms: TIME_BUDGET_MS,
        abort_signal: Some(Arc::clone(abort)),
        ..Default::default()
    };
    let result = picker.grep(&parsed, &options);

    let hits: Vec<SearchResult> = result
        .matches
        .iter()
        .filter_map(|m| {
            let file = result.files.get(m.file_index)?;
            let rel_path = PathBuf::from(file.relative_path(picker));
            Some(SearchResult {
                path: root.join(&rel_path),
                rel_path,
                line_number: usize::try_from(m.line_number).unwrap_or(usize::MAX),
                line: m.line_content.clone(),
            })
        })
        .collect();

    #[cfg(feature = "debug")]
    debug_log!(
        "grep {:?} for {query:?}: {} hit(s) in {} file(s) of {}, {:.3}ms",
        root,
        hits.len(),
        result.files_with_matches,
        result.total_files_searched,
        start.elapsed().as_secs_f64() * 1000.0,
    );

    hits
}
