use std::time::Instant;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use fff_search::file_picker::FilePicker;
use fff_search::grep::{GrepMode, GrepSearchOptions, parse_grep_query};

#[cfg(feature = "debug")]
use crate::debug_log;
use crate::file_mask::FileMask;
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

/// Grep the indexed files for `query` as literal text, keeping only the files
/// `mask` admits.
pub fn grep(
    picker: &FilePicker,
    root: &Path,
    query: &str,
    mask: &FileMask,
    abort: &Arc<AtomicBool>,
) -> Vec<SearchResult> {
    let start = Instant::now();

    let parsed = parse_grep_query(query);
    let mut hits: Vec<SearchResult> = Vec::new();
    let mut file_offset = 0;

    #[cfg(feature = "debug")]
    let (mut files_with_matches, mut total_files_searched) = (0, 0);

    loop {
        // The page limit is applied before lfm sees the matches, so a mask that
        // admits few files needs further pages to fill a batch. Each page gets
        // what is left of the overall budget, keeping the total bounded.
        let elapsed = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        let options = GrepSearchOptions {
            file_offset,
            page_limit: MAX_RESULTS,
            mode: GrepMode::PlainText,
            time_budget_ms: TIME_BUDGET_MS.saturating_sub(elapsed),
            abort_signal: Some(Arc::clone(abort)),
            ..Default::default()
        };
        let result = picker.grep(&parsed, &options);

        #[cfg(feature = "debug")]
        {
            files_with_matches += result.files_with_matches;
            total_files_searched += result.total_files_searched;
        }

        hits.extend(result.matches.iter().filter_map(|m| {
            let file = result.files.get(m.file_index)?;
            let rel_path = PathBuf::from(file.relative_path(picker));
            if !mask.matches(&rel_path) {
                return None;
            }
            Some(SearchResult {
                path: root.join(&rel_path),
                rel_path,
                line_number: usize::try_from(m.line_number).unwrap_or(usize::MAX),
                line: m.line_content.clone(),
            })
        }));

        // An unmasked query is answered by the first page, exactly as before.
        if mask.is_empty()
            || hits.len() >= MAX_RESULTS
            || result.next_file_offset == 0
            || abort.load(Ordering::Relaxed)
            || elapsed >= TIME_BUDGET_MS
        {
            break;
        }
        file_offset = result.next_file_offset;
    }

    hits.truncate(MAX_RESULTS);

    #[cfg(feature = "debug")]
    debug_log!(
        "grep {:?} for {query:?}: {} hit(s) in {files_with_matches} file(s) of \
         {total_files_searched}, {:.3}ms",
        root,
        hits.len(),
        start.elapsed().as_secs_f64() * 1000.0,
    );

    hits
}
