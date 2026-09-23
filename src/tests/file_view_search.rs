use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

use crate::keys::{InputMode, input_mode, to_message};
use crate::message::{EditOp, Field, Message, SearchKind, Surface};
use crate::model::{Model, ViewContent};
use crate::state::PersistedState;
use crate::ui::file_panel;
use crate::update::update;

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("lfm-view-search-{}-{}", std::process::id(), n));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A model with `a.txt` holding `contents`, the viewer open on it and focused —
/// where the search keys live.
fn viewer_on(contents: &str) -> Model {
    let dir = temp_dir();
    fs::write(dir.join("a.txt"), contents).unwrap();
    let mut model = Model::init(PersistedState::default()).unwrap();
    model.left_files = file_panel::Model::init(dir).unwrap();
    model.left_files.selection = 0;
    let (model, _) = update(model, Message::ViewFile);
    let (model, _) = update(model, Message::NextPanel);
    assert!(
        model.file_view_focus.has_keys(),
        "viewer should have the keys"
    );
    model
}

/// The message `code` produces for `model`'s current input mode.
fn key_message(model: &Model, code: KeyCode) -> Option<Message> {
    let event = Event::Key(KeyEvent::new(code, KeyModifiers::NONE));
    to_message(&event, model.active_panel, &input_mode(model))
}

/// Draw the model once, so the viewer records the body size its search needs.
fn draw(model: &mut Model, width: u16, height: u16) {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| crate::view::view(model, f)).unwrap();
}

/// Draw the model once and return what reached the screen.
fn draw_dump(model: &mut Model, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| crate::view::view(model, f)).unwrap();
    format!("{:?}", terminal.backend().buffer())
}

/// One turn of the event loop: handle `msg`, then draw, the way `main` does.
/// The draw is what tells the viewer how big its body is, which is what the
/// search needs to place its matches and to scroll to one.
fn step(model: Model, msg: Message) -> Model {
    let (mut model, _) = update(model, msg);
    draw(&mut model, 80, 24);
    model
}

/// Run `query` in the focused viewer, redrawing between keystrokes as the
/// event loop does.
fn search_for(mut model: Model, query: &str) -> Model {
    draw(&mut model, 80, 24);
    model = step(model, Message::Open(Field::ViewSearch));
    for c in query.chars() {
        model = step(model, Message::Edit(Field::ViewSearch, EditOp::Char(c)));
    }
    step(model, Message::ViewSearchConfirm)
}

fn search(model: &Model) -> &crate::view_search::ViewSearch {
    &model.file_view.as_ref().expect("viewer open").search
}

/// `/` in the focused viewer opens the query row and hands it the keys, so what
/// follows is typed into it rather than scrolling the file.
#[test]
fn slash_opens_the_query_row_in_the_focused_viewer() {
    let model = viewer_on("alpha\n");
    let msg = key_message(&model, KeyCode::Char('/')).expect("/ should open the search");
    assert!(matches!(msg, Message::Open(Field::ViewSearch)));

    let (model, _) = update(model, msg);
    assert!(search(&model).input.active);
    assert!(matches!(input_mode(&model), InputMode::FileViewSearch));

    // The query row takes the keys: `j` types a character instead of scrolling.
    let msg = key_message(&model, KeyCode::Char('j')).expect("j should reach the query row");
    assert!(matches!(
        msg,
        Message::Edit(Field::ViewSearch, EditOp::Char('j'))
    ));
}

/// With the file list focused, `/` still opens the panel filter: the viewer
/// only claims the key while it holds the keys itself.
#[test]
fn slash_still_filters_while_the_file_list_has_the_focus() {
    let model = viewer_on("alpha\n");
    let (model, _) = update(model, Message::NextPanel);
    assert!(!model.file_view_focus.has_keys());

    let msg = key_message(&model, KeyCode::Char('/')).expect("/ should open the filter");
    assert!(matches!(msg, Message::Open(Field::Filter)));
}

/// A confirmed query finds every occurrence, in reading order, ignoring case.
#[test]
fn confirming_a_query_finds_every_match() {
    let model = viewer_on("alpha beta\ngamma\nAlpha again\n");
    let model = search_for(model, "alpha");

    let search = search(&model);
    assert_eq!(search.query, "alpha");
    assert_eq!(search.matches.len(), 2);
    assert_eq!(search.matches[0].row, 0);
    assert_eq!(search.matches[0].col, 0);
    assert_eq!(search.matches[1].row, 2);
    assert_eq!(search.current, 0, "the first match is the current one");
    assert!(!search.input.active, "the query row hands the keys back");
}

/// Two matches on one line are both reported, at the columns they are drawn at.
#[test]
fn matches_on_one_line_report_their_columns() {
    let model = viewer_on("one two one\n");
    let model = search_for(model, "one");

    let search = search(&model);
    assert_eq!(search.matches.len(), 2);
    assert_eq!((search.matches[0].row, search.matches[0].col), (0, 0));
    assert_eq!((search.matches[1].row, search.matches[1].col), (0, 8));
    assert_eq!(search.matches[0].width, 3);
}

/// `n` steps forward and wraps at the end; `Shift+N` steps back and wraps at
/// the start.
#[test]
fn n_and_shift_n_step_through_the_matches() {
    let model = viewer_on("x\nx\nx\n");
    let model = search_for(model, "x");
    assert_eq!(search(&model).matches.len(), 3);

    let msg = key_message(&model, KeyCode::Char('n')).expect("n should step forward");
    assert!(matches!(msg, Message::ViewSearchNext));
    let (model, _) = update(model, msg);
    assert_eq!(search(&model).current, 1);

    let (model, _) = update(model, Message::ViewSearchNext);
    let (model, _) = update(model, Message::ViewSearchNext);
    assert_eq!(search(&model).current, 0, "next wraps past the last match");

    let (model, _) = update(model, Message::ViewSearchPrev);
    assert_eq!(
        search(&model).current,
        2,
        "prev wraps back to the last match"
    );
}

/// A query nothing matches stays standing, with nothing to step through — the
/// panel says so rather than silently dropping what was typed.
#[test]
fn a_query_with_no_match_stays_standing() {
    let model = viewer_on("alpha\n");
    let mut model = search_for(model, "zeta");
    assert_eq!(search(&model).query, "zeta");
    assert!(search(&model).matches.is_empty());

    let dump = draw_dump(&mut model, 80, 24);
    assert!(dump.contains("/zeta"), "{dump}");
    assert!(dump.contains("no match"), "{dump}");
}

/// Confirming an empty query drops the search instead of leaving an empty row.
#[test]
fn confirming_an_empty_query_clears_the_search() {
    let model = viewer_on("alpha\n");
    let (model, _) = update(model, Message::Open(Field::ViewSearch));
    let (model, _) = update(model, Message::ViewSearchConfirm);
    assert!(!search(&model).is_visible());
}

/// Esc on the query row cancels the search and leaves the viewer open.
#[test]
fn esc_on_the_query_row_cancels_the_search() {
    let model = viewer_on("alpha\n");
    let model = search_for(model, "alpha");
    let (model, _) = update(model, Message::Open(Field::ViewSearch));

    let msg = key_message(&model, KeyCode::Esc).expect("Esc should cancel the search");
    assert!(matches!(msg, Message::Cancel(Field::ViewSearch)));

    let (model, _) = update(model, msg);
    assert!(model.file_view.is_some(), "the viewer stays open");
    assert!(!search(&model).is_visible());
    assert!(search(&model).matches.is_empty());
}

/// Esc in the viewer itself still closes it, search standing or not.
#[test]
fn esc_in_the_viewer_still_closes_it() {
    let model = viewer_on("alpha\n");
    let model = search_for(model, "alpha");

    let msg = key_message(&model, KeyCode::Esc).expect("Esc should close the viewer");
    assert!(matches!(msg, Message::Close(Surface::FileView)));

    let (model, _) = update(model, msg);
    assert!(model.file_view.is_none());
}

/// A search belongs to the file it was run on: moving the file list to another
/// file reloads the viewer, and the query goes with the old contents.
#[test]
fn moving_to_another_file_drops_the_search() {
    let dir = temp_dir();
    fs::write(dir.join("a.txt"), "alpha\n").unwrap();
    fs::write(dir.join("b.txt"), "alpha\n").unwrap();
    let mut model = Model::init(PersistedState::default()).unwrap();
    model.left_files = file_panel::Model::init(dir).unwrap();
    model.left_files.selection = 0;
    let (model, _) = update(model, Message::ViewFile);
    let (model, _) = update(model, Message::NextPanel);
    let model = search_for(model, "alpha");
    assert!(!search(&model).matches.is_empty());

    let (model, _) = update(model, Message::NextPanel);
    let (model, _) = update(
        model,
        Message::Nav(Surface::Panel, crate::message::NavOp::Down),
    );
    assert_eq!(model.file_view.as_ref().unwrap().name, "b.txt");
    assert!(!search(&model).is_visible());
}

/// A match below the fold scrolls onto the screen, a couple of rows down from
/// the top of the panel so what leads up to it is still readable.
#[test]
fn a_match_below_the_fold_scrolls_into_view() {
    let mut body = String::new();
    for i in 0..200 {
        use std::fmt::Write;
        writeln!(body, "line {i}").unwrap();
    }
    body.push_str("needle here\n");
    let mut model = search_for(viewer_on(&body), "needle");

    let row = search(&model).matches[0].row;
    assert!(row > 20, "the match must start off screen");

    let view = model.file_view.as_ref().unwrap();
    let height = usize::from(view.viewport_height);
    let ViewContent::Text(state) = &view.content else {
        panic!("expected a text view");
    };
    let scroll = state.scroll();
    assert!(
        scroll <= row && row < scroll + height,
        "the match should be on screen: scroll {scroll}, row {row}, height {height}"
    );

    let dump = draw_dump(&mut model, 80, 24);
    assert!(dump.contains("needle here"), "{dump}");
    assert!(dump.contains("/needle"), "{dump}");
    assert!(dump.contains("1/1"), "{dump}");
}

/// A match already on screen leaves the scroll where it is: confirming a query
/// must not jump the text the user is reading.
#[test]
fn a_visible_match_does_not_scroll_the_view() {
    let model = search_for(viewer_on("alpha\nbeta\n"), "beta");
    let ViewContent::Text(state) = &model.file_view.as_ref().unwrap().content else {
        panic!("expected a text view");
    };
    assert_eq!(state.scroll(), 0);
}

/// `/` does nothing on an image: there is no text to look through.
#[test]
fn slash_does_nothing_on_an_image() {
    let dir = temp_dir();
    image::RgbaImage::new(2, 2)
        .save(dir.join("pic.png"))
        .unwrap();
    let mut model = Model::init(PersistedState::default()).unwrap();
    model.left_files = file_panel::Model::init(dir).unwrap();
    model.left_files.selection = 0;
    model.picker = Some(ratatui_image::picker::Picker::halfblocks());

    let (model, _) = update(model, Message::ViewFile);
    let (model, _) = update(model, Message::NextPanel);
    let (model, _) = update(model, Message::Open(Field::ViewSearch));
    assert!(!search(&model).is_visible());
}

/// The fuzzy finder still answers `f` from the file list while a viewer search
/// stands, so the two searches stay distinct commands.
#[test]
fn the_file_list_keeps_its_own_search_keys() {
    let model = viewer_on("alpha\n");
    let model = search_for(model, "alpha");
    let (model, _) = update(model, Message::NextPanel);

    let msg = key_message(&model, KeyCode::Char('f')).expect("f should open the finder");
    assert!(matches!(msg, Message::SearchOpen(SearchKind::Files)));
}
