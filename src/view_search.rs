//! Text search inside the viewer panel: the `/` query row, the matches it
//! found, and which of them is current.
//!
//! Matches are located in the *rendered* text — the same styled, pre-wrapped
//! [`Text`] the widget draws — rather than in the file's bytes. That is what
//! makes a match position mean something: the viewer scrolls by rendered rows,
//! and a Markdown or hex view puts a byte offset nowhere near the row it ends
//! up on. The cost is one extra render of the file per search, and one more
//! whenever the panel's width changes.

use ratatui::text::{Span, Text};

use crate::model::ViewContent;
use crate::ui::input_box;

/// Rows of context kept above a match the viewer scrolls to, so it does not
/// land on the very first row of the panel.
const CONTEXT_ROWS: usize = 2;

/// One match of the query, in rendered rows and display columns, so the
/// highlight lands on exactly the cells the widget drew.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Match {
    /// Rendered row the match sits on, counted from the top of the file.
    pub row: usize,
    /// Display column within that row, from the left edge of the text area.
    pub col: u16,
    /// Display width of the matched text.
    pub width: u16,
}

/// The viewer's search: the query row, the standing query, and its matches.
pub struct ViewSearch {
    /// The `/` query row while it has the keys.
    pub input: input_box::Model,
    /// The confirmed query. Empty when no search stands, which is also what
    /// tells the panel not to draw the search row.
    pub query: String,
    /// Every match of `query`, in reading order.
    pub matches: Vec<Match>,
    /// Index into `matches` of the one the viewer is on.
    pub current: usize,
    /// Rendered width `matches` were found at. Rows and columns only mean
    /// anything at that width, so a resize recomputes them.
    pub width: u16,
}

impl ViewSearch {
    pub fn new() -> Self {
        Self {
            input: input_box::Model::new(),
            query: String::new(),
            matches: Vec::new(),
            current: 0,
            width: 0,
        }
    }

    /// Put the query row on screen, empty.
    pub fn open(&mut self) {
        self.input.open();
    }

    /// Drop the query row and everything the last query found.
    pub fn clear(&mut self) {
        self.input.close();
        self.query.clear();
        self.matches.clear();
        self.current = 0;
        self.width = 0;
    }

    /// Whether the panel gives up a row for the search: while the query row has
    /// the keys, and while a query stands.
    pub fn is_visible(&self) -> bool {
        self.input.active || !self.query.is_empty()
    }

    /// The match the viewer is on, if there is one.
    pub fn current_match(&self) -> Option<Match> {
        self.matches.get(self.current).copied()
    }

    /// Put the cursor on the first match at or after `row`, wrapping to the
    /// first match when every one of them is above it.
    pub fn select_from_row(&mut self, row: usize) {
        self.current = self
            .matches
            .iter()
            .position(|m| m.row >= row)
            .unwrap_or_default();
    }

    /// Move to the next match, wrapping at the end.
    pub fn select_next(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.current = (self.current + 1) % self.matches.len();
    }

    /// Move to the previous match, wrapping at the start.
    pub fn select_prev(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.current = self
            .current
            .checked_sub(1)
            .unwrap_or(self.matches.len() - 1);
    }

    /// The row to scroll to so the current match is on screen with a little
    /// context above it, or `None` when the match is already visible in a
    /// `height`-row panel scrolled to `scroll`.
    pub fn scroll_target(&self, scroll: usize, height: usize) -> Option<usize> {
        let row = self.current_match()?.row;
        if row >= scroll && row < scroll + height {
            return None;
        }
        Some(row.saturating_sub(CONTEXT_ROWS))
    }
}

/// Find every match of the standing query in `content` rendered `width` columns
/// wide, keeping the cursor on a match that still exists.
///
/// An image has no text to search, and a zero width means the panel has not
/// been drawn yet; both leave the search with no matches.
pub fn recompute(search: &mut ViewSearch, content: &ViewContent, width: u16) {
    search.matches.clear();
    search.width = width;
    let ViewContent::Text(state) = content else {
        return;
    };
    if width == 0 || search.query.is_empty() {
        return;
    }
    let text = state.view().render_bytes(state.bytes(), width);
    search.matches = find(&text, &search.query);
    search.current = search.current.min(search.matches.len().saturating_sub(1));
}

/// Every match of `query` in `text`, in reading order.
///
/// Matching is literal and case-insensitive over ASCII: folding only ASCII
/// keeps the match's byte length equal to the query's, so the columns it
/// reports stay exact whatever the file's encoding of the surrounding text.
fn find(text: &Text<'_>, query: &str) -> Vec<Match> {
    let needle = query.to_ascii_lowercase();
    let mut out = Vec::new();
    // An empty needle matches everywhere and advances nowhere; no query stands
    // for it, so there is nothing to report.
    if needle.is_empty() {
        return out;
    }
    for (row, line) in text.lines.iter().enumerate() {
        let rendered: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        let haystack = rendered.to_ascii_lowercase();
        let mut at = 0;
        while let Some(found) = haystack[at..].find(&needle) {
            let start = at + found;
            let end = start + needle.len();
            out.push(Match {
                row,
                col: display_width(&rendered[..start]),
                width: display_width(&rendered[start..end]),
            });
            // A match never overlaps the one before it, so the scan resumes
            // after it rather than one byte on.
            at = end;
        }
    }
    out
}

/// Columns `s` occupies on screen, which is what a column position in a
/// rendered row is counted in.
fn display_width(s: &str) -> u16 {
    u16::try_from(Span::raw(s).width()).unwrap_or(u16::MAX)
}
