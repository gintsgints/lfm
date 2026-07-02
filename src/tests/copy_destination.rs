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
        Effect::StartCopy(_, dest) => {
            assert_eq!(dest, dst, "destination should be the panel folder")
        }
        _ => panic!("expected StartCopy effect, destination not retargeted into subdir"),
    }
}
