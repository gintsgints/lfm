//! Cleanup of `capture`-mode command output before it reaches the TUI.
//!
//! Captured bytes are arbitrary: a child that thinks it is talking to a
//! terminal emits colour codes, cursor moves, carriage returns and tabs. Those
//! bytes end up in the ratatui buffer verbatim and are written straight back to
//! the terminal, which moves the cursor and corrupts the screen. Everything a
//! terminal would act on is stripped here, leaving plain text lines.

/// Columns a tab advances to. Matches the terminal default.
const TAB_WIDTH: usize = 8;

/// Strip escape sequences and control characters, expand tabs, and normalise
/// line endings so the output is safe to hand to a `Paragraph`.
pub fn sanitize(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    // Column within the current line, so tabs land on real tab stops.
    let mut col = 0usize;

    while let Some(c) = chars.next() {
        match c {
            '\x1b' => skip_escape(&mut chars),
            '\n' => {
                out.push('\n');
                col = 0;
            }
            // A lone CR rewrites the line in a terminal (progress meters). We
            // have no cursor to move, so drop it; CRLF keeps its LF.
            '\r' => {}
            '\t' => {
                let pad = TAB_WIDTH - (col % TAB_WIDTH);
                for _ in 0..pad {
                    out.push(' ');
                }
                col += pad;
            }
            // Remaining C0 controls and DEL have no printable meaning.
            c if c.is_control() => {}
            c => {
                out.push(c);
                col += 1;
            }
        }
    }
    out
}

/// Consume the rest of an escape sequence that starts at an already-read ESC.
///
/// Handles CSI (`ESC [ … final`), OSC (`ESC ] … BEL` or `ESC ] … ESC \`), the
/// other string-terminated introducers, and plain two-character escapes.
fn skip_escape(chars: &mut std::iter::Peekable<std::str::Chars>) {
    let Some(intro) = chars.next() else {
        return;
    };
    match intro {
        '[' => {
            // Parameter and intermediate bytes, then one final byte 0x40..=0x7e.
            for c in chars.by_ref() {
                if matches!(c, '\x40'..='\x7e') {
                    break;
                }
            }
        }
        ']' | 'P' | 'X' | '^' | '_' => {
            // String sequence: runs until BEL or ST (ESC \).
            while let Some(c) = chars.next() {
                match c {
                    '\x07' => break,
                    '\x1b' => {
                        if chars.peek() == Some(&'\\') {
                            chars.next();
                        }
                        break;
                    }
                    _ => {}
                }
            }
        }
        // Intermediate byte (charset designators like `ESC ( B`): the final
        // byte follows and must go too, or it prints as stray text.
        '\x20'..='\x2f' => {
            chars.next();
        }
        // Two-character escape (RIS, index, …): already consumed.
        _ => {}
    }
}
