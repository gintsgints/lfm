use crate::capture::sanitize;

/// Colour codes and other CSI sequences are removed; the text between them
/// survives. Left in, the raw ESC bytes reach the terminal and move the cursor.
#[test]
fn strips_csi_sequences() {
    assert_eq!(sanitize("\x1b[31mred\x1b[0m plain"), "red plain");
    assert_eq!(sanitize("\x1b[2J\x1b[1;1Hcleared"), "cleared");
}

/// OSC sequences run until BEL or ST, both of which end the sequence.
#[test]
fn strips_osc_sequences() {
    assert_eq!(sanitize("\x1b]0;title\x07after"), "after");
    assert_eq!(sanitize("\x1b]0;title\x1b\\after"), "after");
}

/// Tabs become spaces up to the next 8-column stop, counted per line.
#[test]
fn expands_tabs_to_tab_stops() {
    assert_eq!(sanitize("a\tb"), "a       b");
    assert_eq!(sanitize("abcdefgh\tx"), "abcdefgh        x");
    assert_eq!(sanitize("ab\tc\nd\te"), "ab      c\nd       e");
}

/// Newlines are the only control character kept: CR is dropped (so CRLF becomes
/// LF) and backspace-style controls go with it.
#[test]
fn normalises_line_endings_and_drops_other_controls() {
    assert_eq!(sanitize("one\r\ntwo\r\n"), "one\ntwo\n");
    assert_eq!(sanitize("50%\r100%\n"), "50%100%\n");
    assert_eq!(sanitize("a\x08b\x00c"), "abc");
}

/// Charset designators carry an intermediate byte before the final one; both
/// go, so `ESC ( B` does not leave a stray `B` in the text.
#[test]
fn strips_charset_designators() {
    assert_eq!(sanitize("\x1b(Bplain"), "plain");
    assert_eq!(sanitize("a\x1b)0b"), "ab");
}

/// A truncated escape at the end of the output is dropped rather than emitted.
#[test]
fn drops_a_trailing_incomplete_escape() {
    assert_eq!(sanitize("text\x1b"), "text");
    assert_eq!(sanitize("text\x1b[3"), "text");
}
