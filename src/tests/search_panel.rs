use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::file_find::FileFindResult;
use crate::message::{Message, SearchKind};
use crate::model::{ActivePanel, Model};
use crate::search::SearchResult;
use crate::state::PersistedState;
use crate::ui::file_panel;
use crate::update::{Effect, update};

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lfm-panel-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A model over a directory holding `one` and `two`.
fn model_at(dir: &Path) -> Model {
    for name in ["one", "two"] {
        fs::write(dir.join(name), b"").unwrap();
    }
    let mut model = Model::init(PersistedState::default()).unwrap();
    model.left_files = file_panel::Model::init(dir.to_path_buf()).unwrap();
    model
}

fn send(model: Model, msgs: &[Message]) -> (Model, Effect) {
    msgs.iter()
        .fold((model, Effect::None), |(m, _), msg| update(m, *msg))
}

/// How far the popup of `kind` has navigated: whether the query row has the
/// keys, and which result the cursor is on.
fn focus_and_selection(model: &Model, kind: SearchKind) -> Option<(bool, usize)> {
    match kind {
        SearchKind::Content => model
            .content_search
            .as_ref()
            .map(|p| (p.input_focused, p.selection)),
        SearchKind::Files => model
            .file_find
            .as_ref()
            .map(|p| (p.input_focused, p.selection)),
    }
}

/// Give the popup two results without going near the background engine.
fn seed_results(model: &mut Model, kind: SearchKind, dir: &Path) {
    match kind {
        SearchKind::Content => {
            let panel = model.content_search.as_mut().unwrap();
            panel.results = ["one", "two"]
                .iter()
                .map(|n| SearchResult {
                    path: dir.join(n),
                    rel_path: PathBuf::from(n),
                    line_number: 1,
                    line: String::new(),
                })
                .collect();
        }
        SearchKind::Files => {
            let panel = model.file_find.as_mut().unwrap();
            panel.results = ["one", "two"]
                .iter()
                .map(|n| FileFindResult {
                    path: dir.join(n),
                    rel_path: PathBuf::from(n),
                })
                .collect();
        }
    }
}

const KINDS: [SearchKind; 2] = [SearchKind::Content, SearchKind::Files];

/// Opening roots the popup at the active panel's directory and asks for the
/// index up front.
#[test]
fn opening_roots_the_popup_and_prepares_the_index() {
    for kind in KINDS {
        let dir = temp_dir();
        let model = model_at(&dir);
        let (model, effect) = update(model, Message::SearchOpen(kind));

        let root = match kind {
            SearchKind::Content => model.content_search.as_ref().unwrap().root.clone(),
            SearchKind::Files => model.file_find.as_ref().unwrap().root.clone(),
        };
        assert_eq!(root, dir);
        assert!(matches!(
            effect,
            Effect::PrepareContentSearch { .. } | Effect::PrepareFileFind { .. }
        ));
        assert_eq!(focus_and_selection(&model, kind), Some((true, 0)));
    }
}

/// The pinned panel has no directory of its own, so there is nothing to search.
#[test]
fn the_pinned_panel_has_nothing_to_search() {
    for kind in KINDS {
        let dir = temp_dir();
        let mut model = model_at(&dir);
        model.active_panel = ActivePanel::Pinned;
        let (model, effect) = update(model, Message::SearchOpen(kind));

        assert!(matches!(effect, Effect::None));
        assert!(focus_and_selection(&model, kind).is_none());
    }
}

/// Tab hands the keys to the results, Down walks them and clamps at the last
/// one, and Up off the first returns the keys to the query row.
#[test]
fn the_focus_and_the_cursor_walk_the_results() {
    for kind in KINDS {
        let dir = temp_dir();
        let model = model_at(&dir);
        let (mut model, _) = update(model, Message::SearchOpen(kind));
        seed_results(&mut model, kind, &dir);

        let (model, _) = send(model, &[Message::SearchToggleFocus(kind)]);
        assert_eq!(focus_and_selection(&model, kind), Some((false, 0)));

        // Two results, so the second Down has nowhere left to go.
        let (model, _) = send(
            model,
            &[Message::SearchDown(kind), Message::SearchDown(kind)],
        );
        assert_eq!(focus_and_selection(&model, kind), Some((false, 1)));

        let (model, _) = send(model, &[Message::SearchUp(kind), Message::SearchUp(kind)]);
        assert_eq!(focus_and_selection(&model, kind), Some((true, 0)));
    }
}

/// Confirming a result closes the popup and leaves the cursor on that file in
/// the left panel.
#[test]
fn confirming_reveals_the_result_in_the_left_panel() {
    for kind in KINDS {
        let dir = temp_dir();
        let model = model_at(&dir);
        let (mut model, _) = update(model, Message::SearchOpen(kind));
        seed_results(&mut model, kind, &dir);

        let (model, _) = send(
            model,
            &[Message::SearchToggleFocus(kind), Message::SearchDown(kind)],
        );
        let (model, effect) = update(model, Message::SearchConfirm(kind));

        assert!(matches!(effect, Effect::None));
        assert!(focus_and_selection(&model, kind).is_none());
        assert!(matches!(model.active_panel, ActivePanel::LeftFiles));
        assert_eq!(model.left_files.current_dir, dir);
        let name = model
            .left_files
            .visible_entries()
            .nth(model.left_files.selection)
            .map(|(_, e)| e.name.clone());
        assert_eq!(name.as_deref(), Some("two"));
    }
}

/// Enter with nothing to confirm leaves the popup up.
#[test]
fn confirming_without_a_result_keeps_the_popup_open() {
    for kind in KINDS {
        let dir = temp_dir();
        let model = model_at(&dir);
        let (model, _) = update(model, Message::SearchOpen(kind));
        let (model, _) = update(model, Message::SearchConfirm(kind));

        assert!(focus_and_selection(&model, kind).is_some());
    }
}

#[test]
fn cancelling_closes_the_popup() {
    for kind in KINDS {
        let dir = temp_dir();
        let model = model_at(&dir);
        let (model, _) = update(model, Message::SearchOpen(kind));
        let (model, _) = update(model, Message::SearchCancel(kind));

        assert!(focus_and_selection(&model, kind).is_none());
    }
}
