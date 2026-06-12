use std::sync::mpsc;

use crate::file_find::{FileFindMsg, run_file_find};

/// A fuzzy query with no separators still matches the dotted file name.
#[test]
fn finds_file_by_fuzzy_name() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let (tx, rx) = mpsc::channel();
    run_file_find(&root, "mainrs", &tx);

    let mut paths = Vec::new();
    while let Ok(msg) = rx.recv() {
        match msg {
            FileFindMsg::Hit(r) => paths.push(r.rel_path.display().to_string()),
            FileFindMsg::Done => break,
        }
    }

    assert!(
        paths.iter().any(|p| p.ends_with("main.rs")),
        "expected main.rs in fuzzy results, got: {paths:?}"
    );
}
