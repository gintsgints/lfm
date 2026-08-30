use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::keys::{InputMode, to_message};
use crate::message::Message;
use crate::model::{ActivePanel, CommandPicker, Model};
use crate::presets::{CommandTemplate, OutputMode, Preset};
use crate::state::PersistedState;
use crate::ui::file_panel;
use crate::update::{Effect, update};

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lfm-shortcut-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn preset(label: &str, key: Option<char>) -> Preset {
    Preset {
        label: label.to_owned(),
        command: CommandTemplate::Argv(vec!["true".to_owned()]),
        output: OutputMode::Background,
        key,
    }
}

/// A model with the picker up over a real directory holding one file, so a
/// preset that runs has both a cwd and a selection.
fn model_with_presets(presets: Vec<Preset>) -> Model {
    let dir = temp_dir();
    fs::write(dir.join("a"), b"hello").unwrap();

    let mut model = Model::init(PersistedState::default()).unwrap();
    model.left_files = file_panel::Model::init(dir).unwrap();
    model.command_picker = Some(CommandPicker::new(presets));
    model
}

fn char_key(c: char) -> Option<Message> {
    let event = Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    to_message(&event, ActivePanel::LeftFiles, &InputMode::CommandPicker)
}

/// Characters typed in the picker are not resolved by `keys.rs` — which presets
/// are loaded is model state, so the whole char goes to `update`.
#[test]
fn picker_chars_become_shortcut_messages() {
    assert!(matches!(
        char_key('f'),
        Some(Message::CommandPickerShortcut('f'))
    ));
    assert!(matches!(
        char_key('j'),
        Some(Message::CommandPickerShortcut('j'))
    ));
}

#[test]
fn bound_key_runs_that_preset_without_moving_first() {
    let model = model_with_presets(vec![
        preset("first", Some('a')),
        preset("second", Some('b')),
    ]);
    let (model, effect) = update(model, Message::CommandPickerShortcut('b'));

    let Effect::RunCommand { spec } = effect else {
        panic!("expected the preset to run");
    };
    assert_eq!(spec.label, "second");
    // Running a preset closes the picker.
    assert!(model.command_picker.is_none());
}

#[test]
fn bound_key_is_case_sensitive() {
    let model = model_with_presets(vec![preset("lower", Some('r')), preset("upper", Some('R'))]);
    let (_, effect) = update(model, Message::CommandPickerShortcut('R'));

    let Effect::RunCommand { spec } = effect else {
        panic!("expected the preset to run");
    };
    assert_eq!(spec.label, "upper");
}

/// A preset that needs `{input}` still goes through the prompt step when
/// started from its shortcut.
#[test]
fn bound_key_on_an_input_preset_opens_the_prompt() {
    let mut p = preset("grep", Some('g'));
    p.command = CommandTemplate::Argv(vec!["grep".to_owned(), "{input}".to_owned()]);
    let model = model_with_presets(vec![p]);

    let (model, effect) = update(model, Message::CommandPickerShortcut('g'));
    assert!(matches!(effect, Effect::None));
    let cp = model.command_picker.as_ref().expect("picker stays open");
    assert!(cp.input.is_some());
}

/// `j`/`k` keep navigating as long as no preset claims them.
#[test]
fn unbound_j_and_k_still_navigate() {
    let model = model_with_presets(vec![preset("first", Some('a')), preset("second", None)]);

    let (model, _) = update(model, Message::CommandPickerShortcut('j'));
    assert_eq!(model.command_picker.as_ref().unwrap().selection, 1);

    let (model, _) = update(model, Message::CommandPickerShortcut('k'));
    assert_eq!(model.command_picker.as_ref().unwrap().selection, 0);
}

/// An explicit binding outranks the built-in navigation — the arrow keys are
/// still there for moving.
#[test]
fn a_preset_bound_to_j_wins_over_navigation() {
    let model = model_with_presets(vec![preset("first", None), preset("jump", Some('j'))]);
    let (_, effect) = update(model, Message::CommandPickerShortcut('j'));

    let Effect::RunCommand { spec } = effect else {
        panic!("expected the preset to run");
    };
    assert_eq!(spec.label, "jump");
}

/// A character no preset claims and that isn't `j`/`k` does nothing at all.
#[test]
fn unbound_key_is_ignored() {
    let model = model_with_presets(vec![preset("first", Some('a'))]);
    let (model, effect) = update(model, Message::CommandPickerShortcut('z'));

    assert!(matches!(effect, Effect::None));
    let cp = model.command_picker.as_ref().expect("picker stays open");
    assert_eq!(cp.selection, 0);
    assert!(cp.input.is_none());
}

/// Two presets sharing a key is a config mistake, not a crash: the first wins.
#[test]
fn duplicate_keys_run_the_first_match() {
    let model = model_with_presets(vec![
        preset("first", Some('d')),
        preset("second", Some('d')),
    ]);
    let (_, effect) = update(model, Message::CommandPickerShortcut('d'));

    let Effect::RunCommand { spec } = effect else {
        panic!("expected the preset to run");
    };
    assert_eq!(spec.label, "first");
}
