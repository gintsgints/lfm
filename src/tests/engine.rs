use super::{src_dir, wait_for_content, wait_for_files};
use crate::engine::{Kind, SearchEngine};

/// One index answers both a content search and a file find — the tree is walked
/// once, not once per panel.
#[test]
fn one_index_serves_both_panels() {
    let mut engine = SearchEngine::spawn(src_dir());

    engine.search(Kind::Content, "pub struct SearchEngine".to_owned());
    let hits = wait_for_content(&engine);
    assert!(
        hits.iter().any(|r| r.rel_path.ends_with("engine.rs")),
        "expected the declaration in engine.rs"
    );

    // Indexing finished for the first query, so the fuzzy pass reuses it.
    assert!(!engine.is_indexing());
    engine.search(Kind::Files, "mainrs".to_owned());
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

    engine.search(Kind::Content, "fn".to_owned());
    engine.search(Kind::Content, "fn main".to_owned());
    engine.search(Kind::Files, "mainrs".to_owned());

    assert_eq!(engine.generation(Kind::Content), 2);
    assert_eq!(engine.generation(Kind::Files), 1);
}
