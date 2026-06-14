use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode,
};

use crate::keys::{is_key_release, normalize_key_event, shift_held_change};

fn shifted(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::SHIFT))
}

fn normalized_char(event: Event) -> Option<char> {
    match normalize_key_event(event, true) {
        Event::Key(key) => match key.code {
            KeyCode::Char(c) => Some(c),
            _ => None,
        },
        _ => None,
    }
}

/// A shifted lowercase letter folds to its uppercase form.
#[test]
fn folds_shifted_letter_to_uppercase() {
    assert_eq!(normalized_char(shifted(KeyCode::Char('s'))), Some('S'));
}

/// Shift+`/` is `?` — it must fold so `?` stays a distinct command from `/`.
#[test]
fn folds_shifted_slash_to_question_mark() {
    assert_eq!(normalized_char(shifted(KeyCode::Char('/'))), Some('?'));
}

/// A few other shifted punctuation keys reach their US-keyboard symbols.
#[test]
fn folds_shifted_punctuation() {
    assert_eq!(normalized_char(shifted(KeyCode::Char('1'))), Some('!'));
    assert_eq!(normalized_char(shifted(KeyCode::Char(';'))), Some(':'));
    assert_eq!(normalized_char(shifted(KeyCode::Char('`'))), Some('~'));
}

/// Without the extended-keys flag the event passes through untouched.
#[test]
fn passes_through_when_extended_keys_disabled() {
    let event = shifted(KeyCode::Char('/'));
    assert_eq!(normalize_key_event(event.clone(), false), event);
}

/// A key with no distinct shifted form (e.g. `Enter`) is left alone.
#[test]
fn leaves_non_char_keys_untouched() {
    let event = shifted(KeyCode::Enter);
    assert_eq!(normalize_key_event(event.clone(), true), event);
}

fn shift_key(kind: KeyEventKind) -> Event {
    let mut key = KeyEvent::new(
        KeyCode::Modifier(ModifierKeyCode::LeftShift),
        KeyModifiers::SHIFT,
    );
    key.kind = kind;
    Event::Key(key)
}

/// Pressing or releasing Shift reports the held state for the hint bar.
#[test]
fn reports_shift_press_and_release() {
    assert_eq!(
        shift_held_change(&shift_key(KeyEventKind::Press)),
        Some(true)
    );
    assert_eq!(
        shift_held_change(&shift_key(KeyEventKind::Repeat)),
        Some(true)
    );
    assert_eq!(
        shift_held_change(&shift_key(KeyEventKind::Release)),
        Some(false)
    );
}

/// Non-Shift keys do not affect the tracked Shift state.
#[test]
fn ignores_shift_state_for_other_keys() {
    let event = Event::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    assert_eq!(shift_held_change(&event), None);
}

/// Key releases are flagged so they don't re-trigger a command.
#[test]
fn detects_key_release() {
    let mut key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    key.kind = KeyEventKind::Release;
    assert!(is_key_release(&Event::Key(key)));

    let press = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    assert!(!is_key_release(&Event::Key(press)));
}
