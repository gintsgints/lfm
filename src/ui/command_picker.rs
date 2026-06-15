use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};

use crate::model::CommandPicker;
use crate::theme;
use crate::ui::input_box;

pub fn render(frame: &mut Frame, area: Rect, picker: &CommandPicker) {
    let popup_area = centered_rect(70, 70, area);

    let title = if picker.input.is_some() {
        " Run command — input "
    } else {
        " Run command "
    };

    let key = |s: &'static str| Span::styled(s, Style::default().fg(theme::ACTIVE_BORDER));
    let dim = |s: &'static str| Span::styled(s, Style::default().fg(theme::INACTIVE_BORDER));

    let bottom = if picker.input.is_some() {
        Line::from(vec![
            key("[Enter]"),
            dim(" run  "),
            key("[Esc]"),
            dim(" back "),
        ])
    } else {
        Line::from(vec![
            key("[Enter]"),
            dim(" select  "),
            key("[j/k]"),
            dim(" move  "),
            key("[Esc]"),
            dim(" close "),
        ])
    };

    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(theme::TEXT)))
        .title_bottom(bottom)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::POPUP_BORDER));

    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    if let Some(input) = &picker.input {
        let prompt_label = picker
            .presets
            .get(picker.selection)
            .map_or("input", |p| p.label.as_str());
        render_input(frame, inner, input, prompt_label);
    } else {
        render_list(frame, inner, picker);
    }
}

fn render_list(frame: &mut Frame, area: Rect, picker: &CommandPicker) {
    if picker.presets.is_empty() {
        let msg = Line::from(Span::styled(
            "  No presets configured. Edit ~/.config/lfm/commands.json.",
            Style::default().fg(theme::INACTIVE_BORDER),
        ));
        frame.render_widget(ratatui::widgets::Paragraph::new(msg), area);
        return;
    }

    // Reserve roughly a third of the width for the dim command preview.
    let preview_width = area.width.saturating_sub(2) / 3;
    let label_width = area.width.saturating_sub(preview_width).saturating_sub(4) as usize;

    let items: Vec<ListItem> = picker
        .presets
        .iter()
        .map(|p| {
            let label = truncate(&p.label, label_width.max(1));
            let preview = truncate(&p.command_preview(), preview_width as usize);
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("  {label:<label_width$}  "),
                    Style::default().fg(theme::TEXT),
                ),
                Span::styled(preview, Style::default().fg(theme::INACTIVE_BORDER)),
            ]))
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .bg(theme::HIGHLIGHT_BG)
            .fg(theme::HIGHLIGHT_FG)
            .add_modifier(Modifier::BOLD),
    );

    let mut state = ListState::default();
    state.select(Some(picker.selection.min(picker.presets.len() - 1)));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_input(frame: &mut Frame, area: Rect, input: &input_box::Model, prompt_label: &str) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(area);

    let header = Line::from(Span::styled(
        format!("  {prompt_label}"),
        Style::default().fg(theme::ACTIVE_BORDER),
    ));
    frame.render_widget(ratatui::widgets::Paragraph::new(header), chunks[0]);

    let prompt_style = Style::default().fg(theme::ACTIVE_BORDER);
    let text_style = Style::default()
        .fg(theme::TEXT)
        .add_modifier(Modifier::BOLD);
    let cursor_style = text_style.add_modifier(Modifier::UNDERLINED);

    let before = input.text[..input.cursor()].to_owned();
    let (cursor_span, after_span) = if input.cursor() < input.text.len() {
        let c = input.text[input.cursor()..].chars().next().unwrap();
        let end = input.cursor() + c.len_utf8();
        (
            Span::styled(input.text[input.cursor()..end].to_owned(), cursor_style),
            Span::styled(input.text[end..].to_owned(), text_style),
        )
    } else {
        (
            Span::styled("_", cursor_style),
            Span::styled(String::new(), text_style),
        )
    };

    let line = Line::from(vec![
        Span::styled("  > ", prompt_style),
        Span::styled(before, text_style),
        cursor_span,
        after_span,
    ]);
    frame.render_widget(ratatui::widgets::Paragraph::new(line), chunks[1]);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else if max <= 1 {
        s.chars().take(max).collect()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}

fn centered_rect(width_percent: u16, height_percent: u16, area: Rect) -> Rect {
    let margin_h = (100 - width_percent) / 2;
    let margin_v = (100 - height_percent) / 2;
    let vertical = Layout::vertical([
        Constraint::Percentage(margin_v),
        Constraint::Percentage(height_percent),
        Constraint::Percentage(margin_v),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage(margin_h),
        Constraint::Percentage(width_percent),
        Constraint::Percentage(margin_h),
    ])
    .split(vertical[1])[1]
}
