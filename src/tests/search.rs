use super::{src_dir, wait_for_content};
use crate::engine::{Kind, SearchEngine};

/// Grep finds the matching line, and the index built for the first query is
/// reused by the second — the engine only ever indexes once per root.
#[test]
fn greps_file_contents_and_reuses_the_index() {
    let mut engine = SearchEngine::spawn(src_dir());

    engine.search(Kind::Content, "pub struct SearchResult".to_owned());
    assert_eq!(engine.generation(Kind::Content), 1);
    let hits = wait_for_content(&engine);
    assert!(
        hits.iter().any(
            |r| r.rel_path.ends_with("search.rs") && r.line.contains("pub struct SearchResult")
        ),
        "expected the declaration in search.rs"
    );

    // The index is complete by the time the first batch lands, so the second
    // query is answered without another filesystem walk.
    assert!(!engine.is_indexing());
    engine.search(Kind::Content, "TIME_BUDGET_MS".to_owned());
    assert_eq!(engine.generation(Kind::Content), 2);
    let hits = wait_for_content(&engine);
    assert!(
        hits.iter().any(|r| r.rel_path.ends_with("search.rs")),
        "expected hits in search.rs"
    );
    assert!(!engine.is_indexing());
}

/// Line numbers and absolute paths point back at the matched file.
#[test]
fn reports_line_number_and_absolute_path() {
    let mut engine = SearchEngine::spawn(src_dir());

    engine.search(Kind::Content, "TIME_BUDGET_MS: u64".to_owned());
    let hits = wait_for_content(&engine);
    let hit = hits
        .iter()
        .find(|r| r.rel_path.ends_with("search.rs"))
        .expect("expected a hit in search.rs");
    assert!(hit.line_number > 0);
    assert!(
        hit.path.is_absolute() && hit.path.exists(),
        "{:?}",
        hit.path
    );
}

/// A query with no match reports an empty batch rather than hanging.
#[test]
fn reports_an_empty_batch_when_nothing_matches() {
    let mut engine = SearchEngine::spawn(src_dir());

    // Assembled at runtime: a literal would match this very file, which sits
    // inside the indexed root.
    engine.search(Kind::Content, format!("{}_{}", "zzz", "no-such-content"));
    assert!(wait_for_content(&engine).is_empty());
}
