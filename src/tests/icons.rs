use crate::icons::{self, FILE};

/// Known extensions map to their dedicated glyph (not the generic fallback).
#[test]
fn known_extensions_get_specific_icons() {
    assert_eq!(icons::for_name("main.rs"), "\u{e7a8}");
    assert_eq!(icons::for_name("script.py"), "\u{e606}");
    assert_eq!(icons::for_name("photo.png"), "\u{f1c5}");
    assert_eq!(icons::for_name("archive.zip"), "\u{f1c6}");
    assert_ne!(icons::for_name("notes.md"), FILE);
}

/// Extension matching ignores case.
#[test]
fn extension_match_is_case_insensitive() {
    assert_eq!(icons::for_name("IMAGE.PNG"), icons::for_name("image.png"));
    assert_eq!(icons::for_name("Cargo.TOML"), icons::for_name("cargo.toml"));
}

/// Unknown and extensionless names fall back to the generic file glyph.
#[test]
fn unknown_and_extensionless_fall_back() {
    assert_eq!(icons::for_name("data.unknownext"), FILE);
    assert_eq!(icons::for_name("README"), FILE);
    assert_eq!(icons::for_name(""), FILE);
}
