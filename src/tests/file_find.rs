use super::{root_dir, src_dir, wait_for_files};
use crate::engine::{Kind, SearchEngine};

/// A fuzzy query with no separators still matches the dotted file name, and the
/// index built for the first query is reused by the second — the engine only
/// ever indexes once per root.
#[test]
fn finds_file_by_fuzzy_name_and_reuses_the_index() {
    let mut engine = SearchEngine::spawn(src_dir());

    engine.search(Kind::Files, "mainrs".to_owned(), String::new());
    assert_eq!(engine.generation(Kind::Files), 1);
    let hits = wait_for_files(&engine);
    assert!(
        hits.iter().any(|r| r.rel_path.ends_with("main.rs")),
        "expected main.rs in fuzzy results"
    );

    // No second filesystem walk: the index is already built.
    assert!(!engine.is_indexing());
    engine.search(Kind::Files, "enginers".to_owned(), String::new());
    assert_eq!(engine.generation(Kind::Files), 2);
    let hits = wait_for_files(&engine);
    assert!(
        hits.iter().any(|r| r.rel_path.ends_with("engine.rs")),
        "expected engine.rs in fuzzy results"
    );
    assert!(!engine.is_indexing());
}

/// Results carry an absolute path so the panel can navigate to the file.
#[test]
fn reports_absolute_paths() {
    let mut engine = SearchEngine::spawn(src_dir());

    engine.search(Kind::Files, "mainrs".to_owned(), String::new());
    let hits = wait_for_files(&engine);
    let hit = hits
        .iter()
        .find(|r| r.rel_path.ends_with("main.rs"))
        .expect("expected main.rs in fuzzy results");
    assert!(
        hit.path.is_absolute() && hit.path.exists(),
        "{:?}",
        hit.path
    );
}

/// A name mask keeps only the matching files; a path mask anchors the hits to a
/// subdirectory of the root.
#[test]
fn a_mask_restricts_which_names_are_ranked() {
    let mut engine = SearchEngine::spawn(root_dir());

    engine.search(Kind::Files, "cargo".to_owned(), "*.toml".to_owned());
    let hits = wait_for_files(&engine);
    assert!(!hits.is_empty(), "expected Cargo.toml");
    assert!(
        hits.iter()
            .all(|r| r.rel_path.extension().is_some_and(|e| e == "toml")),
        "expected only .toml paths, got {:?}",
        hits.iter().map(|r| r.rel_path.clone()).collect::<Vec<_>>()
    );

    engine.search(Kind::Files, "panel".to_owned(), "src/ui/*.rs".to_owned());
    let hits = wait_for_files(&engine);
    assert!(!hits.is_empty(), "expected panels under src/ui");
    assert!(
        hits.iter().all(|r| r.rel_path.starts_with("src/ui")),
        "expected only paths under src/ui, got {:?}",
        hits.iter().map(|r| r.rel_path.clone()).collect::<Vec<_>>()
    );
}
