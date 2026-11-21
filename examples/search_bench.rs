//! Times lfm's content search on a directory, for comparison with ripgrep.
//!
//! ```bash
//! cargo run --release --example search_bench -- <root> <query>... [--iters N] [--mask GLOBS]
//! ```
//!
//! Reports, per query:
//! - `app`: the exact call the content panel makes (`search::grep`, 200-hit
//!   cap, 2 s budget), so it compares with `rg ... | head -200`.
//! - `full`: every page with no cap or budget, so it compares with a complete
//!   `rg --count-matches` run.
//!
//! `--mask` applies to both, like the panel's file mask (`rg -g`).

use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

use fff_search::file_picker::{FilePicker, FilePickerOptions};
use fff_search::grep::{GrepMode, GrepSearchOptions, parse_grep_query};

#[path = "../src/file_mask.rs"]
mod file_mask;
// The bench only counts hits; it never reads their fields.
#[allow(dead_code)]
#[path = "../src/search.rs"]
mod search;

use file_mask::FileMask;

struct Args {
    root: PathBuf,
    queries: Vec<String>,
    iters: usize,
    mask: String,
}

fn parse_args() -> Args {
    let mut root = None;
    let mut queries = Vec::new();
    let mut iters = 10;
    let mut mask = String::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iters" => iters = args.next().and_then(|n| n.parse().ok()).unwrap_or(iters),
            "--mask" => mask = args.next().unwrap_or_default(),
            _ if root.is_none() => root = Some(PathBuf::from(arg)),
            _ => queries.push(arg),
        }
    }
    let Some(root) = root.filter(|_| !queries.is_empty()) else {
        eprintln!("usage: search_bench <root> <query>... [--iters N] [--mask GLOBS]");
        std::process::exit(2);
    };
    Args {
        root,
        queries,
        iters,
        mask,
    }
}

/// Same options as the engine's `build_index`.
fn build_index(root: &Path) -> FilePicker {
    let options = FilePickerOptions {
        base_path: root.to_string_lossy().into_owned(),
        watch: false,
        enable_home_dir_scanning: true,
        ..Default::default()
    };
    let mut picker = FilePicker::new(options).expect("cannot index");
    picker.collect_files().expect("cannot index");
    picker
}

/// Every match in the index: all pages, no hit cap, no time budget.
fn full_scan(picker: &FilePicker, query: &str, mask: &FileMask) -> (usize, usize) {
    let parsed = parse_grep_query(query);
    let (mut matches, mut files) = (0, 0);
    let mut file_offset = 0;
    loop {
        let options = GrepSearchOptions {
            file_offset,
            page_limit: usize::MAX,
            max_matches_per_file: usize::MAX,
            mode: GrepMode::PlainText,
            ..Default::default()
        };
        let result = picker.grep(&parsed, &options);
        let admitted: Vec<bool> = result
            .files
            .iter()
            .map(|f| mask.matches(Path::new(&f.relative_path(picker))))
            .collect();
        matches += result
            .matches
            .iter()
            .filter(|m| admitted[m.file_index])
            .count();
        files += admitted.iter().filter(|&&a| a).count();
        if result.next_file_offset == 0 {
            return (matches, files);
        }
        file_offset = result.next_file_offset;
    }
}

fn time<T>(iters: usize, mut run: impl FnMut() -> T) -> (Duration, Duration, T) {
    let mut out = run(); // warm-up, also the reported result
    let mut samples: Vec<Duration> = (0..iters)
        .map(|_| {
            let start = Instant::now();
            out = run();
            start.elapsed()
        })
        .collect();
    samples.sort();
    (samples[0], samples[samples.len() / 2], out)
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn main() {
    let args = parse_args();
    let root = std::fs::canonicalize(&args.root).expect("bad root");
    let iters = args.iters.max(1);

    let start = Instant::now();
    let picker = build_index(&root);
    println!(
        "index  {:>9.1} ms  {} files  {}",
        ms(start.elapsed()),
        picker.get_files().len(),
        root.display()
    );

    let mask = FileMask::parse(&args.mask);
    let abort = Arc::new(AtomicBool::new(false));
    for query in &args.queries {
        let (min, med, hits) = time(iters, || {
            search::grep(&picker, &root, query, &mask, &abort).len()
        });
        println!(
            "app    {:>9.2} ms min {:>9.2} ms med  {hits:>7} hits             {query:?}",
            ms(min),
            ms(med)
        );
        let (min, med, (matches, files)) = time(iters, || full_scan(&picker, query, &mask));
        println!(
            "full   {:>9.2} ms min {:>9.2} ms med  {matches:>7} hits {files:>6} files {query:?}",
            ms(min),
            ms(med)
        );
    }
}
