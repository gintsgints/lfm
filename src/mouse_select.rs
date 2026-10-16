//! Mouse text selection that stays inside one panel.
//!
//! The terminal's own selection runs across the whole screen, so dragging over
//! the viewer also picks up the file list beside it. lfm captures the mouse
//! instead: a drag selects only within the panel it started in, the selection is
//! drawn reversed, and on release its text goes to the clipboard over OSC 52.

use std::io;

use crossterm::clipboard::CopyToClipboard;
use ratatui::{
    buffer::Buffer,
    crossterm::{
        event::{
            DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent, KeyModifiers, MouseButton,
            MouseEvent, MouseEventKind,
        },
        execute,
    },
    layout::{Margin, Position, Rect},
    style::Modifier,
};
use unicode_width::UnicodeWidthStr;

use crate::message::Message;

/// Have the terminal report the mouse to lfm instead of selecting on its own.
/// Most terminals still select natively with Shift (Option in some macOS
/// terminals) held.
pub fn enable_capture() {
    let _ = execute!(io::stdout(), EnableMouseCapture);
}

pub fn disable_capture() {
    let _ = execute!(io::stdout(), DisableMouseCapture);
}

/// Put `text` on the system clipboard through the terminal (OSC 52), which
/// works over SSH too. A terminal without OSC 52 support ignores it.
pub fn copy_to_clipboard(text: &str) {
    let _ = execute!(io::stdout(), CopyToClipboard::to_clipboard_from(text));
}

/// What a mouse event means to lfm.
pub enum MouseInput {
    Select(Message),
    /// The wheel. With the mouse captured the terminal no longer turns it into
    /// arrow keys itself, so lfm does, and every list scrolls as before.
    Key(KeyEvent),
    Ignore,
}

pub fn translate(event: MouseEvent) -> MouseInput {
    let at = Position::new(event.column, event.row);
    let key = |code| MouseInput::Key(KeyEvent::new(code, KeyModifiers::NONE));
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => MouseInput::Select(Message::MousePress(at)),
        MouseEventKind::Drag(MouseButton::Left) => MouseInput::Select(Message::MouseDrag(at)),
        MouseEventKind::Up(MouseButton::Left) => MouseInput::Select(Message::MouseRelease(at)),
        MouseEventKind::ScrollUp => key(KeyCode::Up),
        MouseEventKind::ScrollDown => key(KeyCode::Down),
        _ => MouseInput::Ignore,
    }
}

/// Where panels were drawn on the last frame, and the selection being made.
#[derive(Default)]
pub struct MouseSelect {
    /// Content areas (inside the border) of the panels drawn on the last
    /// frame, in drawing order: a popup comes after the panel it covers.
    panels: Vec<Rect>,
    selection: Option<Selection>,
    /// Text the view read off the screen for a finished drag, waiting for the
    /// event loop to put it on the clipboard.
    copied: Option<String>,
}

struct Selection {
    /// The panel content the selection is confined to.
    area: Rect,
    /// Where the drag started.
    anchor: Position,
    /// Where the drag is now, or ended.
    head: Position,
    /// The button is up: the text is to be copied on the next frame.
    released: bool,
}

impl MouseSelect {
    /// Forget the panels of the previous frame; the view registers the new ones.
    pub fn begin_frame(&mut self) {
        self.panels.clear();
    }

    /// Record that a bordered panel was drawn over `area`.
    pub fn register(&mut self, area: Rect) {
        let inner = area.inner(Margin::new(1, 1));
        if !inner.is_empty() {
            self.panels.push(inner);
        }
    }

    /// Start a selection in the topmost panel under the pointer. Clicking
    /// outside every panel just drops the old selection.
    pub fn press(&mut self, at: Position) {
        self.selection = self
            .panels
            .iter()
            .rev()
            .find(|area| area.contains(at))
            .map(|&area| Selection {
                area,
                anchor: at,
                head: at,
                released: false,
            });
    }

    /// Follow the pointer, clamped to the panel the drag started in.
    pub fn drag(&mut self, at: Position) {
        if let Some(sel) = self.selection.as_mut()
            && !sel.released
        {
            sel.head = clamp(at, sel.area);
        }
    }

    /// End the drag. A click that never moved selects nothing.
    pub fn release(&mut self, at: Position) {
        self.drag(at);
        match self.selection.as_mut() {
            Some(sel) if sel.anchor != sel.head => sel.released = true,
            _ => self.selection = None,
        }
    }

    pub fn clear(&mut self) {
        self.selection = None;
    }

    /// Text of a finished selection, once the view has read it off the screen.
    pub fn take_copied(&mut self) -> Option<String> {
        self.copied.take()
    }

    /// Read a finished selection's text off `buf`, then draw the selection
    /// reversed. Run after everything else has been drawn.
    pub fn paint(&mut self, buf: &mut Buffer) {
        let Some(sel) = self.selection.as_mut() else {
            return;
        };
        let area = sel.area.intersection(buf.area);
        if area.is_empty() {
            self.selection = None;
            return;
        }
        let (start, end) = ordered(sel.anchor, sel.head);
        if sel.released && self.copied.is_none() {
            self.copied = Some(text(buf, area, start, end));
            // The text is copied once; keep the highlight until the next click.
            sel.released = false;
            sel.anchor = start;
            sel.head = end;
        }
        for row in start.y..=end.y {
            let (from, to) = row_span(area, start, end, row);
            for x in from..=to {
                if let Some(cell) = buf.cell_mut(Position::new(x, row)) {
                    cell.modifier.toggle(Modifier::REVERSED);
                }
            }
        }
    }
}

fn clamp(at: Position, area: Rect) -> Position {
    Position::new(
        at.x.clamp(area.left(), area.right() - 1),
        at.y.clamp(area.top(), area.bottom() - 1),
    )
}

/// The two ends of a selection in reading order.
fn ordered(a: Position, b: Position) -> (Position, Position) {
    if (a.y, a.x) <= (b.y, b.x) {
        (a, b)
    } else {
        (b, a)
    }
}

/// First and last column selected on `row`: the selection runs like text, from
/// `start` to the panel's right edge, whole rows in between, and from the left
/// edge to `end`.
fn row_span(area: Rect, start: Position, end: Position, row: u16) -> (u16, u16) {
    let from = if row == start.y { start.x } else { area.left() };
    let to = if row == end.y {
        end.x
    } else {
        area.right() - 1
    };
    (from, to)
}

/// The characters between `start` and `end`, one line per row, without the
/// padding the panel adds after the text.
fn text(buf: &Buffer, area: Rect, start: Position, end: Position) -> String {
    let mut lines = Vec::new();
    for row in start.y..=end.y {
        let (from, to) = row_span(area, start, end, row);
        let mut line = String::new();
        // A wide character fills the cells after it with padding that is not
        // part of the text. Walk from the panel's edge so a wide character
        // straddling `from` is accounted for.
        let mut covered = 0;
        for x in area.left()..=to {
            let symbol = buf[(x, row)].symbol();
            if covered > 0 {
                covered -= 1;
                continue;
            }
            covered = symbol.width().saturating_sub(1);
            if x >= from {
                line.push_str(symbol);
            }
        }
        lines.push(line.trim_end().to_owned());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::text::Line;

    /// A 12x4 screen with one bordered panel filling it.
    fn screen(rows: &[&str]) -> (Buffer, MouseSelect) {
        let area = Rect::new(0, 0, 12, 4);
        let mut buf = Buffer::empty(area);
        for (i, row) in rows.iter().enumerate() {
            buf.set_line(1, 1 + u16::try_from(i).unwrap(), &Line::raw(*row), 10);
        }
        let mut mouse = MouseSelect::default();
        mouse.begin_frame();
        mouse.register(area);
        (buf, mouse)
    }

    fn select(mouse: &mut MouseSelect, buf: &mut Buffer, from: (u16, u16), to: (u16, u16)) {
        mouse.press(Position::new(from.0, from.1));
        mouse.drag(Position::new(to.0, to.1));
        mouse.release(Position::new(to.0, to.1));
        mouse.paint(buf);
    }

    #[test]
    fn copies_text_within_one_row() {
        let (mut buf, mut mouse) = screen(&["hello world"]);
        select(&mut mouse, &mut buf, (1, 1), (5, 1));
        assert_eq!(mouse.take_copied().as_deref(), Some("hello"));
    }

    #[test]
    fn runs_to_the_panel_edge_between_rows() {
        let (mut buf, mut mouse) = screen(&["one two", "three four"]);
        select(&mut mouse, &mut buf, (5, 1), (5, 2));
        assert_eq!(mouse.take_copied().as_deref(), Some("two\nthree"));
    }

    #[test]
    fn backwards_drag_selects_the_same_text() {
        let (mut buf, mut mouse) = screen(&["one two", "three four"]);
        select(&mut mouse, &mut buf, (5, 2), (5, 1));
        assert_eq!(mouse.take_copied().as_deref(), Some("two\nthree"));
    }

    #[test]
    fn drag_past_the_panel_stays_inside_it() {
        let (mut buf, mut mouse) = screen(&["abc", "def"]);
        // Start on the text, leave through the right border and the bottom.
        select(&mut mouse, &mut buf, (2, 1), (40, 30));
        assert_eq!(mouse.take_copied().as_deref(), Some("bc\ndef"));
    }

    #[test]
    fn wide_characters_are_copied_once() {
        let (mut buf, mut mouse) = screen(&["a界b"]);
        select(&mut mouse, &mut buf, (1, 1), (4, 1));
        assert_eq!(mouse.take_copied().as_deref(), Some("a界b"));
    }

    #[test]
    fn a_plain_click_copies_nothing() {
        let (mut buf, mut mouse) = screen(&["abc"]);
        select(&mut mouse, &mut buf, (2, 1), (2, 1));
        assert_eq!(mouse.take_copied(), None);
    }

    #[test]
    fn click_on_the_border_starts_no_selection() {
        let (mut buf, mut mouse) = screen(&["abc"]);
        select(&mut mouse, &mut buf, (0, 0), (3, 1));
        assert_eq!(mouse.take_copied(), None);
    }

    #[test]
    fn topmost_panel_wins() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 10));
        buf.set_line(5, 5, &Line::raw("popup"), 8);
        let mut mouse = MouseSelect::default();
        mouse.register(Rect::new(0, 0, 20, 10));
        mouse.register(Rect::new(4, 4, 10, 3));
        // Dragging far left stops at the popup's edge, not the panel below.
        select(&mut mouse, &mut buf, (9, 5), (0, 5));
        assert_eq!(mouse.take_copied().as_deref(), Some("popup"));
    }

    #[test]
    fn selection_is_drawn_reversed_and_copied_once() {
        let (mut buf, mut mouse) = screen(&["abc"]);
        select(&mut mouse, &mut buf, (1, 1), (2, 1));
        assert!(buf[(1, 1)].modifier.contains(Modifier::REVERSED));
        assert!(!buf[(3, 1)].modifier.contains(Modifier::REVERSED));
        assert!(mouse.take_copied().is_some());
        let mut next = Buffer::empty(buf.area);
        mouse.paint(&mut next);
        assert_eq!(mouse.take_copied(), None);
        assert!(next[(1, 1)].modifier.contains(Modifier::REVERSED));
    }
}
