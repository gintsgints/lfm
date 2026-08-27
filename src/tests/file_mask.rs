use std::path::Path;

use crate::file_mask::FileMask;

/// A pattern without `/` matches the file name, so it hits at any depth.
#[test]
fn a_name_pattern_matches_at_any_depth() {
    let mask = FileMask::parse("*.rs");
    assert!(mask.matches(Path::new("src/ui/file_panel.rs")));
    assert!(mask.matches(Path::new("build.rs")));
    assert!(!mask.matches(Path::new("Cargo.toml")));
}

/// A pattern with `/` is anchored to the search root, and `*` stays inside one
/// path component.
#[test]
fn a_path_pattern_is_anchored_to_the_root() {
    let mask = FileMask::parse("src/**/*.rs");
    assert!(mask.matches(Path::new("src/ui/file_panel.rs")));
    assert!(!mask.matches(Path::new("build.rs")));

    let shallow = FileMask::parse("src/*.rs");
    assert!(shallow.matches(Path::new("src/main.rs")));
    assert!(!shallow.matches(Path::new("src/ui/file_panel.rs")));
}

/// Commas separate alternatives; an entry passes when any one of them matches.
#[test]
fn comma_separated_patterns_are_alternatives() {
    let mask = FileMask::parse("*.rs, *.toml");
    assert!(mask.matches(Path::new("src/main.rs")));
    assert!(mask.matches(Path::new("Cargo.toml")));
    assert!(!mask.matches(Path::new("README.md")));

    // Name and path forms mix in one mask.
    let mixed = FileMask::parse("src/ui/*.rs,*.toml");
    assert!(mixed.matches(Path::new("src/ui/input_box.rs")));
    assert!(mixed.matches(Path::new("Cargo.toml")));
    assert!(!mixed.matches(Path::new("src/main.rs")));
}

/// An empty mask — and one whose every pattern is unusable — matches
/// everything, so a half-typed mask never blanks the panel.
#[test]
fn an_empty_or_invalid_mask_matches_everything() {
    for text in ["", "   ", " , ,", "["] {
        let mask = FileMask::parse(text);
        assert!(mask.is_empty(), "expected {text:?} to filter nothing");
        assert!(mask.matches(Path::new("src/main.rs")));
    }
}
