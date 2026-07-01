use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::model::{PendingKind, TransferOp};

/// Create a unique empty temp directory for a single test to work in.
fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lfm-overwrite-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn touch(path: &std::path::Path) {
    fs::write(path, b"").unwrap();
}

#[test]
fn no_conflict_when_destination_is_empty() {
    let src_dir = temp_dir();
    let dst_dir = temp_dir();
    let src = src_dir.join("a.txt");
    touch(&src);

    let kind = PendingKind::Copy(vec![src], dst_dir.clone());
    assert!(kind.conflicts().is_empty());

    fs::remove_dir_all(&src_dir).unwrap();
    fs::remove_dir_all(&dst_dir).unwrap();
}

#[test]
fn reports_existing_destination_file() {
    let src_dir = temp_dir();
    let dst_dir = temp_dir();
    let src = src_dir.join("a.txt");
    touch(&src);
    touch(&dst_dir.join("a.txt"));

    let kind = PendingKind::Move(vec![src], dst_dir.clone());
    assert_eq!(kind.conflicts(), vec!["a.txt".to_owned()]);

    fs::remove_dir_all(&src_dir).unwrap();
    fs::remove_dir_all(&dst_dir).unwrap();
}

#[test]
fn only_reports_sources_that_clash() {
    let src_dir = temp_dir();
    let dst_dir = temp_dir();
    let a = src_dir.join("a.txt");
    let b = src_dir.join("b.txt");
    touch(&a);
    touch(&b);
    touch(&dst_dir.join("b.txt"));

    let kind = PendingKind::Copy(vec![a, b], dst_dir.clone());
    assert_eq!(kind.conflicts(), vec!["b.txt".to_owned()]);

    fs::remove_dir_all(&src_dir).unwrap();
    fs::remove_dir_all(&dst_dir).unwrap();
}

#[test]
fn copying_onto_itself_is_not_a_conflict() {
    let dir = temp_dir();
    let src = dir.join("a.txt");
    touch(&src);

    // Destination directory is the source's own directory: the target path
    // equals the source, so it must not be flagged as an overwrite.
    let kind = PendingKind::Copy(vec![src], dir.clone());
    assert!(kind.conflicts().is_empty());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rename_reports_existing_target() {
    let dir = temp_dir();
    let src = dir.join("old.txt");
    let dst = dir.join("new.txt");
    touch(&src);
    touch(&dst);

    let kind = PendingKind::MoveRename(src, dst);
    assert_eq!(kind.conflicts(), vec!["new.txt".to_owned()]);

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rename_to_same_name_is_not_a_conflict() {
    let dir = temp_dir();
    let src = dir.join("a.txt");
    touch(&src);

    let kind = PendingKind::CopyRename(src.clone(), src);
    assert!(kind.conflicts().is_empty());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn op_matches_kind() {
    let p = PathBuf::from("x");
    assert_eq!(PendingKind::Copy(vec![], p.clone()).op(), TransferOp::Copy);
    assert_eq!(
        PendingKind::CopyRename(p.clone(), p.clone()).op(),
        TransferOp::Copy
    );
    assert_eq!(PendingKind::Move(vec![], p.clone()).op(), TransferOp::Move);
    assert_eq!(PendingKind::MoveRename(p.clone(), p).op(), TransferOp::Move);
}
