use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::message::{EditOp, Field, Message};
use crate::ui::file_panel;

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lfm-filter-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A panel over three files: `alpha`, `beta`, `gamma`.
fn panel() -> file_panel::Model {
    let dir = temp_dir();
    for name in ["alpha", "beta", "gamma"] {
        fs::write(dir.join(name), b"").unwrap();
    }
    file_panel::Model::init(dir).unwrap()
}

fn send(model: file_panel::Model, msgs: &[Message]) -> file_panel::Model {
    msgs.iter()
        .fold(model, |m, msg| file_panel::update(m, *msg).0)
}

fn typed(c: char) -> Message {
    Message::Edit(Field::Filter, EditOp::Char(c))
}

fn visible(model: &file_panel::Model) -> Vec<String> {
    model
        .visible_entries()
        .map(|(_, e)| e.name.clone())
        .collect()
}

#[test]
fn typing_narrows_the_list_and_backspace_widens_it() {
    let model = panel();
    let model = send(
        model,
        &[Message::Open(Field::Filter), typed('a'), typed('l')],
    );
    assert_eq!(visible(&model), ["alpha"]);

    // Back to "a", which every one of the three names contains.
    let model = send(model, &[Message::Edit(Field::Filter, EditOp::Backspace)]);
    assert_eq!(visible(&model), ["alpha", "beta", "gamma"]);
}

/// Confirming hands the keys back to the list but keeps the filter applied;
/// only Esc clears it.
#[test]
fn confirm_keeps_the_filter_and_escape_clears_it() {
    let model = panel();
    let model = send(
        model,
        &[
            Message::Open(Field::Filter),
            typed('b'),
            Message::ConfirmFilter,
        ],
    );
    assert!(!model.search.active);
    assert!(model.is_filtering());
    assert_eq!(visible(&model), ["beta"]);

    let model = send(model, &[Message::Cancel(Field::Filter)]);
    assert!(!model.is_filtering());
    assert_eq!(visible(&model), ["alpha", "beta", "gamma"]);
}

/// Re-entering a confirmed filter appends to it rather than starting over, so
/// the cursor has to land after the existing text.
#[test]
fn re_entering_a_filter_appends_to_it() {
    let model = panel();
    let model = send(
        model,
        &[
            Message::Open(Field::Filter),
            typed('a'),
            Message::ConfirmFilter,
            Message::Open(Field::Filter),
            typed('l'),
        ],
    );
    assert_eq!(model.search.text, "al");
    assert_eq!(visible(&model), ["alpha"]);
}

/// Filtering down to a shorter list must not strand the cursor past the end.
#[test]
fn narrowing_re_anchors_the_selection() {
    let mut model = panel();
    model.selection = 2;
    let model = send(model, &[Message::Open(Field::Filter), typed('b')]);
    assert_eq!(model.selection, 0);
    assert_eq!(visible(&model), ["beta"]);
}

/// The filter field carries a cursor like every other text field, so an edit
/// can land in the middle of it.
#[test]
fn the_cursor_moves_within_the_filter() {
    let model = panel();
    let model = send(
        model,
        &[
            Message::Open(Field::Filter),
            typed('l'),
            typed('a'),
            Message::Edit(Field::Filter, EditOp::CursorLeft),
            typed('p'),
            typed('h'),
        ],
    );
    assert_eq!(model.search.text, "lpha");
    assert_eq!(visible(&model), ["alpha"]);
}

/// Moving the cursor changes nothing about which entries are visible, so it
/// must not disturb the selection.
#[test]
fn moving_the_cursor_keeps_the_selection() {
    let mut model = panel();
    model.selection = 1;
    let model = send(
        model,
        &[
            Message::Open(Field::Filter),
            Message::Edit(Field::Filter, EditOp::CursorLeft),
            Message::Edit(Field::Filter, EditOp::CursorRight),
        ],
    );
    assert_eq!(model.selection, 1);
}
