use crate::ui::help_panel::{next_selectable, prev_selectable};

/// Scrolling down skips the blank separator that follows the Navigation group,
/// landing on the next real entry rather than an empty line.
#[test]
fn scrolling_down_skips_blank_separators() {
    // Index 6 is "Shift+Tab"; index 7 is a blank separator; index 8 is the
    // "Selection" section header.
    assert_eq!(next_selectable(6), 8);
}

/// Scrolling up from the top stays at the top instead of underflowing.
#[test]
fn scrolling_up_at_top_stays_put() {
    assert_eq!(prev_selectable(0), 0);
}

/// Scrolling up also skips blank separators.
#[test]
fn scrolling_up_skips_blank_separators() {
    assert_eq!(prev_selectable(8), 6);
}
