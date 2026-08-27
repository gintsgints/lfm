use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::message::Message;
use crate::model::{ActivePanel, Model, TransferMode};
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

/// Confirming a copy or move whose destination is the folder the sources
/// already live in reports it instead of running a transfer that would do
/// nothing — and leaves the destination picker open, so another folder can be
/// chosen once the message is dismissed.
#[test]
fn same_folder_destination_reports_a_message_and_keeps_the_picker_open() {
    for (start, confirm) in [
        (Message::StartCopy, Message::ConfirmCopy),
        (Message::StartMove, Message::ConfirmMove),
    ] {
        let src = temp_dir();
        fs::write(src.join("a.txt"), b"new").unwrap();
        let dst = temp_dir();

        let mut model = Model::init(PersistedState::default()).unwrap();
        model.left_files = file_panel::Model::init(src).unwrap();

        // `start` already points the destination panel at the source folder.
        let (model, _) = update(model, start);
        let (model, effect) = update(model, confirm);

        assert!(
            matches!(effect, Effect::None),
            "no transfer should be launched into the source folder"
        );
        assert!(
            model.error_message.is_some(),
            "the user should be told the target is the source folder"
        );
        assert!(model.progress.is_none(), "no transfer should be running");

        // Dismiss the message and carry on picking a destination.
        let (mut model, _) = update(model, Message::DismissError);
        assert!(model.transfer_mode != TransferMode::None, "still picking");
        assert!(model.active_panel == ActivePanel::RightFiles);
        assert!(model.right_files.dirs_only, "still listing folders only");

        model.right_files.navigate_to(dst.clone());
        let (_model, effect) = update(model, confirm);
        match effect {
            Effect::StartCopy(_, target) | Effect::StartMove(_, target) => {
                assert_eq!(target, dst, "the second destination should be used");
            }
            _ => panic!("expected the transfer to run after choosing another folder"),
        }
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
