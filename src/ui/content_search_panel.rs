use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use crate::model::ContentSearch;
use crate::theme;
use crate::ui::query_row;

pub fn render(frame: &mut Frame, area: Rect, state: &ContentSearch) {
    let popup = centered_rect(90, 80, area);

    let match_label = if state.indexing {
        " indexing\u{2026}  ".to_owned()
    } else if state.query.text.is_empty() {
        String::new()
    } else if state.done {
        format!(" {} matches  ", state.results.len())
    } else {
        format!(" {} matches  searching\u{2026}  ", state.results.len())
    };

    let key = |s: &'static str| Span::styled(s, Style::default().fg(theme::active_border()));
    let dim = |s: &'static str| Span::styled(s, Style::default().fg(theme::inactive_border()));

    let bottom = Line::from(vec![
        Span::styled(match_label, Style::default().fg(theme::inactive_border())),
        key("[\u{2190}/\u{2192}]"),
        dim(" mask  "),
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
            Style::default().fg(theme::text()),
        ))
        .title_bottom(bottom)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::popup_border()));

    let inner = block.inner(popup);
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);

    let chunks = Layout::vertical([
        Constraint::Length(1), // query line
        Constraint::Length(1), // separator
        Constraint::Min(0),    // results
    ])
    .split(inner);

    query_row::render(
        frame,
        chunks[0],
        &state.query,
        &state.mask,
        state.input_focused,
        state.input_field,
    );

    let sep = "─".repeat(usize::from(chunks[1].width));
    frame.render_widget(
        Paragraph::new(Span::styled(
            sep,
            Style::default().fg(theme::inactive_border()),
        )),
        chunks[1],
    );

    render_results(frame, chunks[2], state);
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
            ListItem::new(Span::styled(label, Style::default().fg(theme::text())))
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .bg(theme::highlight_bg())
            .fg(theme::highlight_fg())
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
