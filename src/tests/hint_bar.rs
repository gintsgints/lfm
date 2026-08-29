use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::text::Span;

use crate::keys::{InputMode, to_message};
use crate::model::ActivePanel;
use crate::theme;
use crate::view::{normal_hint_spans, shifted_hint_spans};

/// The key labels in a hint line. Keys are drawn in the active-border colour
/// and their descriptions in the inactive one, which is how they are told
/// apart here.
fn advertised_keys(spans: &[Span<'static>]) -> Vec<String> {
    spans
        .iter()
        .filter(|s| s.style.fg == Some(theme::active_border()))
        .map(|s| s.content.trim().to_owned())
        .collect()
}

/// Whether pressing `label` in a file panel produces any message at all.
fn is_bound(label: &str) -> bool {
    let mut chars = label.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        panic!("hint bar advertises {label:?}, which is not a single key");
    };
    let event = Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    to_message(&event, ActivePanel::LeftFiles, &InputMode::Normal).is_some()
}

/// The hint bar is the only place most of these keys are ever shown, so a key
/// it names must do something. This is the check that was missing when the
/// search, find and sort keys were swapped: the hint bar kept advertising `F`,
/// which had stopped being bound to anything.
#[test]
fn every_key_the_hint_bar_advertises_is_bound() {
    for label in advertised_keys(&normal_hint_spans()) {
        assert!(is_bound(&label), "unshifted hint `{label}` is not bound");
    }
    for label in advertised_keys(&shifted_hint_spans()) {
        assert!(is_bound(&label), "shifted hint `{label}` is not bound");
    }
}

/// The hint bar splits by whether Shift is held, so a key must be listed on
/// the side that actually reaches it.
#[test]
fn the_hint_bar_lists_each_key_on_the_right_side() {
    for label in advertised_keys(&normal_hint_spans()) {
        assert!(
            label.chars().all(|c| !c.is_uppercase()),
            "`{label}` needs Shift but is listed in the unshifted hints"
        );
    }
    for label in advertised_keys(&shifted_hint_spans()) {
        assert!(
            label.chars().any(char::is_uppercase),
            "`{label}` needs no Shift but is listed in the shifted hints"
        );
    }
}
