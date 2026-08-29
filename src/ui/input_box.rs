use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::message::EditOp;
use crate::theme;

pub struct Model {
    pub text: String,
    pub active: bool,
    cursor: usize, // byte offset into `text`
}

impl Model {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            active: false,
            cursor: 0,
        }
    }

    pub fn open(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.active = true;
    }

    pub fn close(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.active = false;
    }

    /// Replace the text and position the cursor at the end.
    pub fn set_text(&mut self, text: String) {
        self.cursor = text.len();
        self.text = text;
    }

    /// Insert `c` at the cursor and advance the cursor.
    pub fn insert(&mut self, c: char) {
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Delete the character immediately before the cursor (backspace semantics).
    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        // Find the start of the preceding char.
        let prev = self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i);
        self.text.remove(prev);
        self.cursor = prev;
    }

    /// Move the cursor one character to the left.
    pub fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor = self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i);
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Put the cursor before the first character.
    pub fn cursor_home(&mut self) {
        self.cursor = 0;
    }

    /// Put the cursor after the last character.
    pub fn cursor_end(&mut self) {
        self.cursor = self.text.len();
    }

    /// Move the cursor one character to the right.
    pub fn move_right(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        let c = self.text[self.cursor..].chars().next().unwrap();
        self.cursor += c.len_utf8();
    }

    /// The text as three spans: everything before the cursor, the character the
    /// cursor sits _on_, and the rest. At end-of-text there is no character to
    /// sit on, so a "_" stands in. Shared by every widget drawing this field.
    pub fn cursor_spans(&self, text_style: Style) -> Vec<Span<'static>> {
        let cursor_style = text_style.add_modifier(Modifier::UNDERLINED);
        let (on_cursor, after) = if self.cursor < self.text.len() {
            let c = self.text[self.cursor..].chars().next().unwrap();
            let end = self.cursor + c.len_utf8();
            (
                self.text[self.cursor..end].to_owned(),
                self.text[end..].to_owned(),
            )
        } else {
            ("_".to_owned(), String::new())
        };
        vec![
            Span::styled(self.text[..self.cursor].to_owned(), text_style),
            Span::styled(on_cursor, cursor_style),
            Span::styled(after, text_style),
        ]
    }
}

/// Apply one editing keystroke. Every text field in the app routes through
/// here, so `Message::Edit` needs no per-field handling of its own.
pub fn apply(field: &mut Model, op: EditOp) {
    match op {
        EditOp::Char(c) => field.insert(c),
        EditOp::Backspace => field.backspace(),
        EditOp::CursorLeft => field.move_left(),
        EditOp::CursorRight => field.move_right(),
    }
}

pub fn render(frame: &mut Frame, area: Rect, model: &Model, label: &str) {
    let popup_area = centered_rect(60, area);

    let block = Block::default()
        .title(Span::styled(
            format!(" {label} "),
            Style::default().fg(theme::text()),
        ))
        .borders(Borders::ALL)
        .style(Style::default().fg(theme::active_border()));

    let text_style = Style::default()
        .fg(theme::text())
        .add_modifier(Modifier::BOLD);
    let content = Line::from(model.cursor_spans(text_style));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(Paragraph::new(content).block(block), popup_area);
}

fn centered_rect(width_percent: u16, area: Rect) -> Rect {
    let margin_h = (100 - width_percent) / 2;
    let vertical = Layout::vertical([
        Constraint::Percentage(45),
        Constraint::Length(3),
        Constraint::Percentage(45),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage(margin_h),
        Constraint::Percentage(width_percent),
        Constraint::Percentage(margin_h),
    ])
    .split(vertical[1])[1]
}
