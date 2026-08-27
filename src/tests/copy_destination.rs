use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::message::Message;
use crate::model::Model;
use crate::state::PersistedState;
use crate::ui::file_panel;
use crate::update::{Effect, update};

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lfm-dest-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The copy destination is the folder shown in the destination panel, even when
/// the cursor happens to rest on a subdirectory inside it.
#[test]
fn destination_is_right_panel_folder_not_highlighted_subdir() {
    let src = temp_dir();
    fs::write(src.join("a.txt"), b"new").unwrap();

    let dst = temp_dir();
    fs::create_dir_all(dst.join("sub")).unwrap(); // cursor lands here; must be ignored

    let mut model = Model::init(PersistedState::default()).unwrap();
    model.left_files = file_panel::Model::init(src).unwrap();

    let (mut model, _) = update(model, Message::StartCopy);
    model.right_files.navigate_to(dst.clone()); // cursor on `sub` (a directory)

    let (_model, effect) = update(model, Message::ConfirmCopy);

    match effect {
        Effect::StartCopy(_, target) => {
            assert_eq!(target, dst, "destination should be the panel folder");
        }
        _ => panic!("expected StartCopy effect, destination not retargeted into subdir"),
    }
}

/// A copy or move destination is always a directory, so the destination panel
/// lists directories only — and goes back to listing everything once the
/// transfer is confirmed or cancelled.
#[test]
fn destination_panel_lists_directories_only() {
    let dst = temp_dir();
    fs::create_dir_all(dst.join("sub")).unwrap();
    fs::write(dst.join("b.txt"), b"old").unwrap();

    for (start, finish) in [
        (Message::StartCopy, Message::CancelCopy),
        (Message::StartMove, Message::CancelMove),
    ] {
        let src = temp_dir();
        fs::write(src.join("a.txt"), b"new").unwrap();

        let mut model = Model::init(PersistedState::default()).unwrap();
        model.left_files = file_panel::Model::init(src).unwrap();

        let (mut model, _) = update(model, start);
        model.right_files.navigate_to(dst.clone());

        let names: Vec<&str> = model
            .right_files
            .visible_entries()
            .map(|(_, e)| e.name.as_str())
            .collect();
        assert_eq!(names, vec!["sub"], "destination panel should hide files");

        // The left panel keeps showing files: it is where sources come from.
        assert!(
            model
                .left_files
                .visible_entries()
                .any(|(_, e)| e.name == "a.txt"),
            "source panel should still list files"
        );

        let (model, _) = update(model, finish);
        assert!(
            !model.right_files.dirs_only,
            "destination panel should list files again after the transfer ends"
        );
    }
}
