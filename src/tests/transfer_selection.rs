use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::message::Message;
use crate::model::Model;
use crate::state::PersistedState;
use crate::ui::file_panel;
use crate::update::update;

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lfm-sel-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn model_at(dir: PathBuf) -> Model {
    let mut model = Model::init(PersistedState::default()).unwrap();
    model.left_files = file_panel::Model::init(dir).unwrap();
    model
}

fn highlighted(model: &Model) -> String {
    let panel = &model.left_files;
    panel
        .visible_entries()
        .nth(panel.selection)
        .map(|(_, e)| e.name.clone())
        .unwrap_or_default()
}

/// A finished copy leaves the cursor on the entry it was copying, not back at
/// the top of the list.
#[test]
fn copied_entry_stays_highlighted() {
    let src = temp_dir();
    for name in ["a", "b", "c"] {
        fs::create_dir_all(src.join(name)).unwrap();
    }

    let mut model = model_at(src.clone());
    model.left_files.selection = 2; // "c"
    assert_eq!(highlighted(&model), "c");

    let (model, _) = update(model, Message::ProgressDone);

    assert_eq!(highlighted(&model), "c");
    fs::remove_dir_all(&src).unwrap();
}

/// When the highlighted entry is gone — moved away, deleted — the cursor keeps
/// its position in the list rather than jumping to the first entry.
#[test]
fn moved_entry_leaves_cursor_in_place() {
    let src = temp_dir();
    for name in ["a", "b", "c"] {
        fs::create_dir_all(src.join(name)).unwrap();
    }

    let mut model = model_at(src.clone());
    model.left_files.selection = 1; // "b"
    fs::remove_dir_all(src.join("b")).unwrap(); // the move took it elsewhere

    let (model, _) = update(model, Message::ProgressDone);

    assert_eq!(model.left_files.selection, 1);
    assert_eq!(highlighted(&model), "c");
    fs::remove_dir_all(&src).unwrap();
}

/// Removing the last entry clamps the cursor to the new end of the list.
#[test]
fn cursor_clamps_when_the_list_shrinks() {
    let src = temp_dir();
    for name in ["a", "b", "c"] {
        fs::create_dir_all(src.join(name)).unwrap();
    }

    let mut model = model_at(src.clone());
    model.left_files.selection = 2; // "c"
    fs::remove_dir_all(src.join("b")).unwrap();
    fs::remove_dir_all(src.join("c")).unwrap();

    let (model, _) = update(model, Message::ProgressDone);

    assert_eq!(model.left_files.selection, 0);
    assert_eq!(highlighted(&model), "a");
    fs::remove_dir_all(&src).unwrap();
}
