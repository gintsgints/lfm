use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::message::{Message, NavOp, Surface};
use crate::ui::{file_panel, pinned_panel};

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lfm-nav-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A panel over three files: `a`, `b`, `c`.
fn panel() -> file_panel::Model {
    let dir = temp_dir();
    for name in ["a", "b", "c"] {
        fs::write(dir.join(name), b"").unwrap();
    }
    file_panel::Model::init(dir).unwrap()
}

fn nav(model: file_panel::Model, ops: &[NavOp]) -> file_panel::Model {
    ops.iter().fold(model, |m, op| {
        file_panel::update(m, Message::Nav(Surface::Panel, *op)).0
    })
}

#[test]
fn the_cursor_moves_and_clamps_at_both_ends() {
    let model = panel();
    let model = nav(model, &[NavOp::Up]);
    assert_eq!(model.selection, 0);

    let model = nav(model, &[NavOp::Down, NavOp::Down, NavOp::Down]);
    assert_eq!(model.selection, 2);
}

/// Marking toggles the entry the cursor leaves, so a run of Shift+Down marks
/// everything it passes over but not the entry it lands on.
#[test]
fn marking_toggles_the_entry_the_cursor_leaves() {
    let model = panel();
    let model = nav(model, &[NavOp::MarkDown, NavOp::MarkDown]);

    assert_eq!(model.selected.iter().copied().collect::<Vec<_>>(), [0, 1]);
    assert_eq!(model.selection, 2);
}

/// Marking is a toggle, so coming back over an entry and marking it again
/// clears it.
#[test]
fn marking_an_entry_twice_unmarks_it() {
    let model = panel();
    // Marks `a` (cursor to `b`), marks `b` (cursor back to `a`), then clears
    // `a` again (cursor to `b`).
    let model = nav(model, &[NavOp::MarkDown, NavOp::MarkUp, NavOp::MarkDown]);

    assert_eq!(model.selected.iter().copied().collect::<Vec<_>>(), [1]);
    assert_eq!(model.selection, 1);
}

/// The file list has no paging, so the page ops leave it alone rather than
/// falling through to some other move.
#[test]
fn the_file_list_ignores_the_page_ops() {
    let model = panel();
    let model = nav(model, &[NavOp::Down]);
    let model = nav(model, &[NavOp::PageUp, NavOp::PageDown]);

    assert_eq!(model.selection, 1);
    assert!(model.selected.is_empty());
}

/// The pinned list moves but has nothing to mark, so Shift+Down does not move
/// its cursor at all — as it did not before.
#[test]
fn the_pinned_list_moves_but_does_not_mark() {
    let pins = vec![PathBuf::from("/one"), PathBuf::from("/two")];
    let model = pinned_panel::Model::with_pins(pins);

    let model = pinned_panel::update(model, Message::Nav(Surface::Panel, NavOp::Down));
    assert_eq!(model.selection, 1);

    let model = pinned_panel::update(model, Message::Nav(Surface::Panel, NavOp::MarkUp));
    assert_eq!(model.selection, 1);
}
