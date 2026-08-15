use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use tui_view::ViewRegistry;

use crate::message::Message;
use crate::model::Model;
use crate::state::PersistedState;
use crate::ui::file_panel;
use crate::update::{detect_text, update};

/// Plain UTF-8 bytes are accepted and returned verbatim.
#[test]
fn accepts_utf8_text() {
    assert_eq!(
        detect_text("hello\nworld".as_bytes().to_vec()),
        Ok("hello\nworld".to_owned())
    );
}

/// An embedded NUL byte marks the content as binary.
#[test]
fn rejects_nul_byte() {
    assert!(detect_text(vec![b'a', 0, b'b']).is_err());
}

/// Invalid UTF-8 (a lone continuation byte) is rejected as binary.
#[test]
fn rejects_invalid_utf8() {
    assert!(detect_text(vec![0xff, 0xfe]).is_err());
}

/// Empty input is valid, empty text.
#[test]
fn accepts_empty() {
    assert_eq!(detect_text(Vec::new()), Ok(String::new()));
}

/// The registry picks a per-format view by extension.
#[test]
fn registry_picks_view_by_extension() {
    let registry = ViewRegistry::with_defaults();
    assert_eq!(
        registry.find(Path::new("README.md")).map(|v| v.name()),
        Some("Markdown")
    );
    assert_eq!(
        registry.find(Path::new("data.json")).map(|v| v.name()),
        Some("JSON")
    );
}

/// An unknown extension matches no view; the viewer falls back to plain text.
#[test]
fn registry_has_no_view_for_unknown_extension() {
    let registry = ViewRegistry::with_defaults();
    assert!(registry.find(Path::new("Makefile")).is_none());
}

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lfm-view-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A model whose left panel shows `a.txt` and `b.txt`, cursor on the first.
fn model_with_two_files() -> Model {
    let dir = temp_dir();
    fs::write(dir.join("a.txt"), b"alpha").unwrap();
    fs::write(dir.join("b.txt"), b"beta").unwrap();
    let mut model = Model::init(PersistedState::default()).unwrap();
    model.left_files = file_panel::Model::init(dir).unwrap();
    model.left_files.selection = 0;
    model
}

/// `v` opens the viewer panel on the highlighted file, and `v` again closes it.
#[test]
fn view_key_toggles_the_panel() {
    let model = model_with_two_files();
    let (model, _) = update(model, Message::ViewFile);
    let view = model.file_view.as_ref().expect("viewer should be open");
    assert_eq!(view.name, "a.txt");
    assert!(
        !model.file_view_focused,
        "focus stays on the file list when the viewer opens"
    );

    let (model, _) = update(model, Message::ViewFile);
    assert!(model.file_view.is_none(), "second `v` closes the viewer");
}

/// Tab moves the focus between the file list and the open viewer instead of
/// switching file panels.
#[test]
fn tab_moves_focus_between_file_list_and_viewer() {
    let model = model_with_two_files();
    let (model, _) = update(model, Message::ViewFile);

    let (model, _) = update(model, Message::NextPanel);
    assert!(model.file_view_focused);

    let (model, _) = update(model, Message::NextPanel);
    assert!(!model.file_view_focused);
}

/// Moving the cursor in the file list reloads the viewer for the new file.
#[test]
fn moving_in_the_file_list_updates_the_viewer() {
    let model = model_with_two_files();
    let (model, _) = update(model, Message::ViewFile);
    assert_eq!(model.file_view.as_ref().unwrap().name, "a.txt");

    let (model, _) = update(model, Message::SelectDown);
    assert_eq!(model.file_view.as_ref().unwrap().name, "b.txt");
}

/// Closing the viewer also drops its focus, so the file list keeps the keys.
#[test]
fn closing_the_viewer_returns_focus_to_the_file_list() {
    let model = model_with_two_files();
    let (model, _) = update(model, Message::ViewFile);
    let (model, _) = update(model, Message::NextPanel);
    let (model, _) = update(model, Message::FileViewClose);
    assert!(model.file_view.is_none());
    assert!(!model.file_view_focused);
}
