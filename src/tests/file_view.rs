use std::path::Path;

use tui_view::ViewRegistry;

use crate::update::detect_text;

/// Plain UTF-8 bytes are accepted and returned verbatim.
#[test]
fn accepts_utf8_text() {
    assert_eq!(
        detect_text("hello\nworld".as_bytes().to_vec()),
        Ok("hello\nworld".to_owned())
    );
}

/// An embedded NUL byte marks the content as binary.
#[test]
fn rejects_nul_byte() {
    assert!(detect_text(vec![b'a', 0, b'b']).is_err());
}

/// Invalid UTF-8 (a lone continuation byte) is rejected as binary.
#[test]
fn rejects_invalid_utf8() {
    assert!(detect_text(vec![0xff, 0xfe]).is_err());
}

/// Empty input is valid, empty text.
#[test]
fn accepts_empty() {
    assert_eq!(detect_text(Vec::new()), Ok(String::new()));
}

/// The registry picks a per-format view by extension.
#[test]
fn registry_picks_view_by_extension() {
    let registry = ViewRegistry::with_defaults();
    assert_eq!(
        registry.find(Path::new("README.md")).map(|v| v.name()),
        Some("Markdown")
    );
    assert_eq!(
        registry.find(Path::new("data.json")).map(|v| v.name()),
        Some("JSON")
    );
}

/// An unknown extension matches no view; the viewer falls back to plain text.
#[test]
fn registry_has_no_view_for_unknown_extension() {
    let registry = ViewRegistry::with_defaults();
    assert!(registry.find(Path::new("Makefile")).is_none());
}
