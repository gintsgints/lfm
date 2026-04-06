use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

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

    /// Move the cursor one character to the right.
    pub fn move_right(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        let c = self.text[self.cursor..].chars().next().unwrap();
        self.cursor += c.len_utf8();
    }
}

pub fn render(frame: &mut Frame, area: Rect, model: &Model, label: &str) {
    let popup_area = centered_rect(60, area);

    let block = Block::default()
        .title(Span::styled(
            format!(" {label} "),
            Style::default().fg(theme::TEXT),
        ))
        .borders(Borders::ALL)
        .style(Style::default().fg(theme::ACTIVE_BORDER));

    let before = &model.text[..model.cursor];
    let text_style = Style::default()
        .fg(theme::TEXT)
        .add_modifier(Modifier::BOLD);
    let cursor_style = text_style.add_modifier(Modifier::UNDERLINED);

    // Cursor sits _on_ the character at the cursor position.
    // At end-of-text there is no character, so show a "_" placeholder.
    let (cursor_span, after_span) = if model.cursor < model.text.len() {
        let c = model.text[model.cursor..].chars().next().unwrap();
        let end = model.cursor + c.len_utf8();
        let on_char = model.text[model.cursor..end].to_owned();
        let after = model.text[end..].to_owned();
        (
            Span::styled(on_char, cursor_style),
            Span::styled(after, text_style),
        )
    } else {
        (
            Span::styled("_", cursor_style),
            Span::styled(String::new(), text_style),
        )
    };

    let content = Line::from(vec![
        Span::styled(before.to_owned(), text_style),
        cursor_span,
        after_span,
    ]);

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
