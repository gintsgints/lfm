#[cfg(feature = "debug")]
use std::time::Instant;
use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::mpsc,
};

#[cfg(feature = "debug")]
use crate::debug_log;

pub struct SearchResult {
    pub path: PathBuf,
    pub rel_path: PathBuf,
    pub line_number: usize,
    pub line: String,
}

pub enum SearchMsg {
    Hit(SearchResult),
    Done,
}

/// Running tally of work done during a search, logged when the search finishes.
#[cfg(feature = "debug")]
#[derive(Default)]
struct SearchStats {
    files: usize,
    hits: usize,
}

pub fn run_search(root: &Path, query: &str, tx: &mpsc::Sender<SearchMsg>) {
    #[cfg(feature = "debug")]
    let start = Instant::now();
    #[cfg(feature = "debug")]
    let mut stats = SearchStats::default();
    search_dir(
        root,
        root,
        query,
        tx,
        #[cfg(feature = "debug")]
        &mut stats,
    );
    #[cfg(feature = "debug")]
    debug_log!(
        "search {:?} for {query:?}: {} hit(s) in {} file(s), {:.3}ms",
        root,
        stats.hits,
        stats.files,
        start.elapsed().as_secs_f64() * 1000.0,
    );
    let _ = tx.send(SearchMsg::Done);
}

fn search_dir(
    root: &Path,
    dir: &Path,
    query: &str,
    tx: &mpsc::Sender<SearchMsg>,
    #[cfg(feature = "debug")] stats: &mut SearchStats,
) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd.filter_map(Result::ok).collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            search_dir(
                root,
                &path,
                query,
                tx,
                #[cfg(feature = "debug")]
                stats,
            );
        } else if ft.is_file() {
            search_file(
                root,
                &path,
                query,
                tx,
                #[cfg(feature = "debug")]
                stats,
            );
        }
    }
}

fn search_file(
    root: &Path,
    path: &Path,
    query: &str,
    tx: &mpsc::Sender<SearchMsg>,
    #[cfg(feature = "debug")] stats: &mut SearchStats,
) {
    let Ok(file) = std::fs::File::open(path) else {
        return;
    };
    #[cfg(feature = "debug")]
    {
        stats.files += 1;
    }
    let reader = BufReader::new(file);
    let rel_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();
    for (i, line_result) in reader.lines().enumerate() {
        let Ok(line) = line_result else {
            // Binary file or encoding error — stop reading this file.
            break;
        };
        if line.contains(query) {
            #[cfg(feature = "debug")]
            {
                stats.hits += 1;
            }
            let hit = SearchMsg::Hit(SearchResult {
                path: path.to_path_buf(),
                rel_path: rel_path.clone(),
                line_number: i + 1,
                line,
            });
            if tx.send(hit).is_err() {
                return; // receiver dropped — search cancelled
            }
        }
    }
}
