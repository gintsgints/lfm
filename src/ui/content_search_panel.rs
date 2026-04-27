use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use crate::model::ContentSearch;
use crate::theme;
use crate::ui::input_box;

pub fn render(frame: &mut Frame, area: Rect, state: &ContentSearch) {
    let popup = centered_rect(90, 80, area);

    let match_label = if state.query.text.is_empty() {
        String::new()
    } else if state.done {
        format!(" {} matches  ", state.results.len())
    } else {
        format!(" {} matches  searching\u{2026}  ", state.results.len())
    };

    let key = |s: &'static str| Span::styled(s, Style::default().fg(theme::ACTIVE_BORDER));
    let dim = |s: &'static str| Span::styled(s, Style::default().fg(theme::INACTIVE_BORDER));

    let bottom = Line::from(vec![
        Span::styled(match_label, Style::default().fg(theme::INACTIVE_BORDER)),
        key("[Tab]"),
        dim(" switch  "),
        key("[Enter]"),
        dim(" select  "),
        key("[Esc]"),
        dim(" cancel "),
    ]);

    let block = Block::default()
        .title(Span::styled(
            " Content search ",
            Style::default().fg(theme::TEXT),
        ))
        .title_bottom(bottom)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::POPUP_BORDER));

    let inner = block.inner(popup);
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);

    let chunks = Layout::vertical([
        Constraint::Length(1), // query line
        Constraint::Length(1), // separator
        Constraint::Min(0),    // results
    ])
    .split(inner);

    render_query_line(frame, chunks[0], &state.query, state.input_focused);

    let sep = "─".repeat(usize::from(chunks[1].width));
    frame.render_widget(
        Paragraph::new(Span::styled(
            sep,
            Style::default().fg(theme::INACTIVE_BORDER),
        )),
        chunks[1],
    );

    render_results(frame, chunks[2], state);
}

fn render_query_line(frame: &mut Frame, area: Rect, query: &input_box::Model, focused: bool) {
    let prompt_style = Style::default().fg(theme::ACTIVE_BORDER);
    let text_style = Style::default()
        .fg(theme::TEXT)
        .add_modifier(Modifier::BOLD);

    let spans = if focused {
        let cursor_style = text_style.add_modifier(Modifier::UNDERLINED);
        let before = query.text[..query.cursor()].to_owned();
        let (cursor_span, after_span) = if query.cursor() < query.text.len() {
            let c = query.text[query.cursor()..].chars().next().unwrap();
            let end = query.cursor() + c.len_utf8();
            (
                Span::styled(query.text[query.cursor()..end].to_owned(), cursor_style),
                Span::styled(query.text[end..].to_owned(), text_style),
            )
        } else {
            (
                Span::styled("_", cursor_style),
                Span::styled(String::new(), text_style),
            )
        };
        vec![
            Span::styled("> ", prompt_style),
            Span::styled(before, text_style),
            cursor_span,
            after_span,
        ]
    } else {
        vec![
            Span::styled("  ", prompt_style),
            Span::styled(
                query.text.clone(),
                Style::default().fg(theme::INACTIVE_BORDER),
            ),
        ]
    };

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_results(frame: &mut Frame, area: Rect, state: &ContentSearch) {
    let items: Vec<ListItem> = state
        .results
        .iter()
        .map(|r| {
            let label = format!(
                "{}:{}: {}",
                r.rel_path.display(),
                r.line_number,
                r.line.trim()
            );
            ListItem::new(Span::styled(label, Style::default().fg(theme::TEXT)))
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .bg(theme::HIGHLIGHT_BG)
            .fg(theme::HIGHLIGHT_FG)
            .add_modifier(Modifier::BOLD),
    );

    let mut list_state = ListState::default();
    if !state.results.is_empty() && !state.input_focused {
        list_state.select(Some(state.selection));
    }

    frame.render_stateful_widget(list, area, &mut list_state);
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
