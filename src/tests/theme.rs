use ratatui::style::Color;

use crate::theme::{parse_hex, to_hex};

/// A leading `#` is optional and the three byte pairs become an RGB color.
#[test]
fn parses_hex_with_and_without_hash() {
    assert_eq!(parse_hex("#cdd6f4"), Some(Color::Rgb(205, 214, 244)));
    assert_eq!(parse_hex("cdd6f4"), Some(Color::Rgb(205, 214, 244)));
}

/// Malformed strings (wrong length, non-hex digits) parse to `None` so the
/// caller can fall back to a default.
#[test]
fn rejects_malformed_hex() {
    assert_eq!(parse_hex(""), None);
    assert_eq!(parse_hex("#fff"), None);
    assert_eq!(parse_hex("#cdd6f"), None);
    assert_eq!(parse_hex("#gggggg"), None);
}

/// `to_hex` and `parse_hex` round-trip an RGB color.
#[test]
fn hex_round_trips() {
    let color = Color::Rgb(243, 139, 168);
    assert_eq!(to_hex(color), "#f38ba8");
    assert_eq!(parse_hex(&to_hex(color)), Some(color));
}
