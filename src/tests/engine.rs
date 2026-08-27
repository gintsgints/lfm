use std::time::Duration;

use super::{src_dir, wait_for_content, wait_for_files};
use crate::engine::{Kind, SearchEngine};
use crate::{INDEX_DEBOUNCE, Index};

/// One index answers both a content search and a file find — the tree is walked
/// once, not once per panel.
#[test]
fn one_index_serves_both_panels() {
    let mut engine = SearchEngine::spawn(src_dir());

    engine.search(
        Kind::Content,
        "pub struct SearchEngine".to_owned(),
        String::new(),
    );
    let hits = wait_for_content(&engine);
    assert!(
        hits.iter().any(|r| r.rel_path.ends_with("engine.rs")),
        "expected the declaration in engine.rs"
    );

    // Indexing finished for the first query, so the fuzzy pass reuses it.
    assert!(!engine.is_indexing());
    engine.search(Kind::Files, "mainrs".to_owned(), String::new());
    let hits = wait_for_files(&engine);
    assert!(
        hits.iter().any(|r| r.rel_path.ends_with("main.rs")),
        "expected main.rs in fuzzy results"
    );
    assert!(!engine.is_indexing());
}

/// Each kind counts its own queries, so a batch is only matched against the
/// panel that asked for it.
#[test]
fn generations_are_tracked_per_kind() {
    let mut engine = SearchEngine::spawn(src_dir());
    assert_eq!(engine.generation(Kind::Content), 0);
    assert_eq!(engine.generation(Kind::Files), 0);

    engine.search(Kind::Content, "fn".to_owned(), String::new());
    engine.search(Kind::Content, "fn main".to_owned(), String::new());
    engine.search(Kind::Files, "mainrs".to_owned(), String::new());

    assert_eq!(engine.generation(Kind::Content), 2);
    assert_eq!(engine.generation(Kind::Files), 1);
}

/// Browsing into a directory only starts indexing once it has stayed put: the
/// walk cannot be cancelled, so a scheduled index must survive the debounce.
#[test]
fn indexing_waits_for_the_directory_to_settle() {
    let mut index = Index::default();

    index.schedule(&src_dir());
    index.tick();
    assert!(
        index.engine.is_none(),
        "must not index before the debounce expires"
    );
    assert!(index.debounce_remaining().is_some());

    std::thread::sleep(INDEX_DEBOUNCE + Duration::from_millis(50));
    index.tick();
    assert!(index.engine.is_some(), "expected the index to start");
    assert!(index.debounce_remaining().is_none());

    // The directory it just indexed is not queued again.
    index.schedule(&src_dir());
    assert!(index.pending.is_none());
}

/// Opening a panel reuses the pre-warmed index, but rebuilds it once lfm has
/// written to the tree.
#[test]
fn stale_index_is_rebuilt_on_panel_open() {
    let mut index = Index::default();

    let engine = index.of(src_dir());
    engine.search(Kind::Content, "fn".to_owned(), String::new());
    assert_eq!(index.of(src_dir()).generation(Kind::Content), 1);

    // A filesystem change means the next open re-walks: the fresh engine has no
    // queries behind it.
    index.stale = true;
    assert_eq!(index.of(src_dir()).generation(Kind::Content), 0);
    assert!(!index.stale);
}
