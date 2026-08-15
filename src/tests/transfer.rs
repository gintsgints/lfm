use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;

use crate::transfer::{ProgressMsg, run_copy, run_move};

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lfm-transfer-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run<F: FnOnce(&mpsc::Sender<ProgressMsg>)>(f: F) {
    let (tx, rx) = mpsc::channel();
    f(&tx);
    drop(tx);
    while rx.recv().is_ok() {}
}

/// Copying a file into its own directory must not truncate it (`fs::copy` onto
/// the same path would otherwise empty it).
#[test]
fn copy_file_onto_itself_preserves_contents() {
    let dir = temp_dir();
    let file = dir.join("a.txt");
    fs::write(&file, b"KEEP").unwrap();

    run(|tx| run_copy(std::slice::from_ref(&file), &dir, tx));

    assert_eq!(fs::read(&file).unwrap(), b"KEEP");
    fs::remove_dir_all(&dir).unwrap();
}

/// Copying a directory into its own parent must not destroy the files inside it.
#[test]
fn copy_dir_into_its_parent_preserves_inner_files() {
    let parent = temp_dir();
    let sub = parent.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("inner.txt"), b"KEEP").unwrap();

    run(|tx| run_copy(std::slice::from_ref(&sub), &parent, tx));

    assert_eq!(fs::read(sub.join("inner.txt")).unwrap(), b"KEEP");
    fs::remove_dir_all(&parent).unwrap();
}

/// Moving a file into its own directory must leave it in place, intact.
#[test]
fn move_file_onto_itself_preserves_contents() {
    let dir = temp_dir();
    let file = dir.join("a.txt");
    fs::write(&file, b"KEEP").unwrap();

    run(|tx| run_move(std::slice::from_ref(&file), &dir, tx));

    assert!(file.exists());
    assert_eq!(fs::read(&file).unwrap(), b"KEEP");
    fs::remove_dir_all(&dir).unwrap();
}

/// A genuine copy into a different directory still overwrites the target there.
#[test]
fn copy_into_other_dir_overwrites_target() {
    let src = temp_dir();
    fs::write(src.join("a.txt"), b"NEW").unwrap();
    let dst = temp_dir();
    fs::write(dst.join("a.txt"), b"OLD").unwrap();

    run(|tx| run_copy(&[src.join("a.txt")], &dst, tx));

    assert_eq!(fs::read(dst.join("a.txt")).unwrap(), b"NEW");
    fs::remove_dir_all(&src).unwrap();
    fs::remove_dir_all(&dst).unwrap();
}
