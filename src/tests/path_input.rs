use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::message::{EditOp, Field, Message};
use crate::ui::file_panel;

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lfm-path-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn send(model: file_panel::Model, msgs: &[Message]) -> file_panel::Model {
    msgs.iter()
        .fold(model, |m, msg| file_panel::update(m, *msg).0)
}

fn typed(field: Field, text: &str) -> Vec<Message> {
    text.chars()
        .map(|c| Message::Edit(field, EditOp::Char(c)))
        .collect()
}

/// The cursor keys reach the new-path dialog: an edit can land in the middle of
/// the name, and the file that gets created is the edited one.
#[test]
fn the_new_path_dialog_edits_at_the_cursor() {
    let dir = temp_dir();
    let model = file_panel::Model::init(dir.clone()).unwrap();

    let mut msgs = vec![Message::NewPath];
    msgs.extend(typed(Field::NewPath, "abc"));
    msgs.push(Message::Edit(Field::NewPath, EditOp::CursorLeft));
    msgs.extend(typed(Field::NewPath, "X"));

    let model = send(model, &msgs);
    assert_eq!(model.new_path_input.text, "abXc");

    let model = send(model, &[Message::NewPathConfirm]);
    assert!(dir.join("abXc").exists());
    assert!(model.new_path_input.text.is_empty());
}

/// Backspace deletes at the cursor, not blindly at the end.
#[test]
fn the_new_path_dialog_backspaces_at_the_cursor() {
    let dir = temp_dir();
    let model = file_panel::Model::init(dir).unwrap();

    let mut msgs = vec![Message::NewPath];
    msgs.extend(typed(Field::NewPath, "abc"));
    msgs.push(Message::Edit(Field::NewPath, EditOp::CursorLeft));
    msgs.push(Message::Edit(Field::NewPath, EditOp::Backspace));

    let model = send(model, &msgs);
    assert_eq!(model.new_path_input.text, "ac");
}

/// The goto dialog shares the same field type, so it edits the same way.
#[test]
fn the_goto_dialog_edits_at_the_cursor() {
    let dir = temp_dir();
    let model = file_panel::Model::init(dir).unwrap();

    let mut msgs = vec![Message::GotoPath];
    msgs.extend(typed(Field::GotoPath, "tmp"));
    msgs.push(Message::Edit(Field::GotoPath, EditOp::CursorLeft));
    msgs.push(Message::Edit(Field::GotoPath, EditOp::CursorLeft));
    msgs.extend(typed(Field::GotoPath, "/"));

    let model = send(model, &msgs);
    assert_eq!(model.goto_input.text, "t/mp");
}
