use crate::model::CaptureView;
use crate::ui::capture_view::{max_scroll, page_step};

fn view(output: &str, width: u16, height: u16) -> CaptureView {
    CaptureView {
        label: "t".into(),
        exit_code: Some(0),
        output: output.to_owned(),
        scroll: 0,
        viewport_width: width,
        viewport_height: height,
    }
}

/// Output that fits the body cannot be scrolled at all.
#[test]
fn output_shorter_than_the_viewport_does_not_scroll() {
    assert_eq!(max_scroll(&view("a\nb\nc\n", 20, 10)), 0);
}

/// The clamp counts rows after wrapping, not logical lines, so the last row of
/// a long line is still reachable.
#[test]
fn wrapped_lines_count_as_multiple_rows() {
    // Two lines of 20 columns wrap to 4 rows in a 10-column body.
    let v = view(&format!("{0}\n{0}\n", "x".repeat(20)), 10, 2);
    assert_eq!(max_scroll(&v), 2);
}

/// Before the first render the viewport is unknown, so every logical line
/// counts as one row and a page is a single row.
#[test]
fn unrendered_view_falls_back_to_logical_lines() {
    assert_eq!(max_scroll(&view("a\nb\nc\n", 0, 0)), 3);
    assert_eq!(page_step(&view("a\nb\nc\n", 0, 0)), 1);
}

/// A page is one screenful minus a row of overlap.
#[test]
fn page_step_is_a_screenful_minus_one_row() {
    assert_eq!(page_step(&view("a", 20, 10)), 9);
}

/// The body text actually reaches the buffer, inside the border.
#[test]
fn renders_the_output_inside_the_block() {
    use ratatui::{Terminal, backend::TestBackend};
    let backend = TestBackend::new(30, 8);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut v = view("hello world\nsecond line\n", 0, 0);
    terminal
        .draw(|f| crate::ui::capture_view::render(f, f.area(), &mut v))
        .unwrap();
    let dump = format!("{:?}", terminal.backend().buffer());
    assert!(dump.contains("hello world"), "{dump}");
    assert!(dump.contains("second line"), "{dump}");
}
