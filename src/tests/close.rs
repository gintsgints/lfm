use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::message::{Message, Surface};
use crate::model::{CaptureView, CommandPicker, Model};
use crate::state::PersistedState;
use crate::ui::file_panel;
use crate::update::update;

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lfm-close-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A model with every dismissable overlay up at once, so a `Close` that hits
/// the wrong one is visible.
fn model_with_overlays() -> Model {
    let dir = temp_dir();
    fs::write(dir.join("a"), b"hello").unwrap();

    let mut model = Model::init(PersistedState::default()).unwrap();
    model.left_files = file_panel::Model::init(dir).unwrap();
    model.capture_view = Some(CaptureView {
        label: "ls".to_owned(),
        exit_code: Some(0),
        output: "out".to_owned(),
        scroll: 0,
        viewport_width: 0,
        viewport_height: 0,
    });
    model.command_picker = Some(CommandPicker::new(Vec::new()));
    let (model, _) = update(model, Message::ViewFile);
    model
}

/// Every overlay now shares one `Close` variant, so each surface must dismiss
/// only itself.
#[test]
fn close_dismisses_only_the_surface_it_names() {
    let model = model_with_overlays();
    assert!(model.capture_view.is_some());
    assert!(model.command_picker.is_some());
    assert!(model.file_view.is_some());

    let (model, _) = update(model, Message::Close(Surface::Capture));
    assert!(model.capture_view.is_none());
    assert!(model.command_picker.is_some());
    assert!(model.file_view.is_some());

    let (model, _) = update(model, Message::Close(Surface::CommandPicker));
    assert!(model.command_picker.is_none());
    assert!(model.file_view.is_some());

    let (model, _) = update(model, Message::Close(Surface::FileView));
    assert!(model.file_view.is_none());
}

/// Closing a surface that is not up is a no-op, not a panic.
#[test]
fn closing_a_surface_that_is_not_up_does_nothing() {
    let dir = temp_dir();
    let mut model = Model::init(PersistedState::default()).unwrap();
    model.left_files = file_panel::Model::init(dir).unwrap();

    for surface in [Surface::Capture, Surface::CommandPicker, Surface::FileView] {
        let (m, _) = update(model, Message::Close(surface));
        model = m;
    }
    assert!(model.capture_view.is_none());
    assert!(model.command_picker.is_none());
    assert!(model.file_view.is_none());
}
