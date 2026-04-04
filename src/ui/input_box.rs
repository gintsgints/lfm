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
}

pub enum Action {
    None,
    Confirmed,
    Cancelled,
}

impl Model {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            active: false,
        }
    }

    pub fn open(&mut self) {
        self.text.clear();
        self.active = true;
    }

    pub fn close(&mut self) {
        self.text.clear();
        self.active = false;
    }
}

pub fn update(
    mut model: Model,
    char_input: Option<char>,
    backspace: bool,
    confirm: bool,
    cancel: bool,
) -> (Model, Action) {
    if cancel {
        model.close();
        return (model, Action::Cancelled);
    }
    if confirm {
        model.active = false;
        return (model, Action::Confirmed);
    }
    if backspace {
        model.text.pop();
    } else if let Some(c) = char_input {
        model.text.push(c);
    }
    (model, Action::None)
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

    let content = Line::from(vec![
        Span::styled(
            model.text.clone(),
            Style::default()
                .fg(theme::TEXT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("█", Style::default().fg(theme::ACTIVE_BORDER)),
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
