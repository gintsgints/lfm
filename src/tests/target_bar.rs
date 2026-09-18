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
    let dir = std::env::temp_dir().join(format!("lfm-bar-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Rows of the rendered screen, top to bottom.
fn rendered_rows(model: &mut Model, width: u16, height: u16) -> Vec<String> {
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut term = ratatui::Terminal::new(backend).unwrap();
    term.draw(|frame| crate::view::view(model, frame)).unwrap();
    let buffer = term.backend().buffer().clone();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}

/// While a copy or a move is picking its destination, the folder it will land
/// in is written across the top of the destination panel.
#[test]
fn the_target_path_is_shown_above_the_destination_panel() {
    for (start, label) in [
        (Message::StartCopy, "copy to"),
        (Message::StartMove, "move to"),
    ] {
        let src = temp_dir();
        fs::write(src.join("a.txt"), b"new").unwrap();
        let dst = temp_dir();
        fs::create_dir_all(dst.join("sub")).unwrap();

        let mut model = Model::init(PersistedState::default()).unwrap();
        model.left_files = file_panel::Model::init(src.clone()).unwrap();

        let (mut model, _) = update(model, start);
        model.right_files.navigate_to(dst.clone());

        let rows = rendered_rows(&mut model, 120, 24);
        let bar = rows
            .iter()
            .position(|row| row.contains(label))
            .expect("the destination bar should be on screen");
        assert!(bar < 2, "the bar is at row {bar}, not on top of the panel");

        // Half a window is narrower than a temp path, so the bar keeps the tail
        // — the folder the transfer lands in.
        let name = dst.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            rows[bar + 1].contains(&name),
            "the bar should spell out the target path, got {:?}",
            rows[bar + 1]
        );
        // It belongs to the right panel, so it starts past the middle column.
        let column = rows[bar].find(label).unwrap();
        assert!(column > 60, "the bar is drawn over the source panel");

        fs::remove_dir_all(&src).unwrap();
        fs::remove_dir_all(&dst).unwrap();
    }
}

/// The bar only shows while a transfer is choosing where to go.
#[test]
fn no_bar_without_a_transfer() {
    let dir = temp_dir();
    let mut model = Model::init(PersistedState::default()).unwrap();
    model.left_files = file_panel::Model::init(dir.clone()).unwrap();

    let rows = rendered_rows(&mut model, 120, 24);
    assert!(
        !rows.iter().any(|row| row.contains("copy to")),
        "no destination bar should be drawn outside a transfer"
    );
    fs::remove_dir_all(&dir).unwrap();
}
