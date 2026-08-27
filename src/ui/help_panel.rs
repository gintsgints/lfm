use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};

use crate::theme;

const KEYBINDINGS: &[(&str, &str)] = &[
    ("Navigation", ""),
    ("j / ↓", "Move down"),
    ("k / ↑", "Move up"),
    ("h / ←", "Go to parent directory"),
    ("l / → / Enter", "Enter directory"),
    ("Tab", "Next panel"),
    ("Shift+Tab", "Previous panel"),
    ("", ""),
    ("Selection", ""),
    ("Shift+J / Shift+↓", "Mark item and move down"),
    ("Shift+K / Shift+↑", "Mark item and move up"),
    ("Esc", "Clear selection"),
    ("", ""),
    ("File operations", ""),
    ("n", "Create file or directory"),
    ("r", "Rename current item"),
    ("g", "Go to path"),
    ("d", "Delete selected or current item"),
    ("c", "Copy selected or current item"),
    ("Shift+C", "Copy single item — rename before placing"),
    ("m", "Move selected or current item"),
    ("Shift+M", "Move single item — rename before placing"),
    ("v", "Toggle viewer panel (follows the file list)"),
    ("Tab (viewer open)", "Switch between file list and viewer"),
    ("Esc (viewer open)", "Close viewer panel"),
    ("e", "Open selected item in $EDITOR"),
    ("o", "Open with default application"),
    ("x", "Run preset command (see ~/.config/lfm/commands.json)"),
    ("Shift+S", "Cycle sort: name / date / ext / size"),
    ("z", "Zip selected or current item(s)"),
    ("u", "Extract .zip or .tar.gz archive"),
    ("s", "Search file contents recursively"),
    ("f", "Fuzzy-find files by name"),
    ("", ""),
    ("Filter", ""),
    ("/", "Enter filter mode"),
    ("↓ / Enter / Tab", "Lock filter and move to file list"),
    ("Esc", "Clear filter"),
    ("", ""),
    ("Pinned directories", ""),
    ("p", "Open pinned panel"),
    ("p (in panel)", "Pin current or selected dir"),
    ("Enter/Space", "Navigate to pinned dir"),
    ("d (in panel)", "Delete pinned dir"),
    ("Esc", "Close pinned panel"),
    ("", ""),
    ("Other", ""),
    ("?", "Show this help"),
    ("q", "Quit"),
];

/// A line is selectable (can hold the highlight cursor) when it carries a key
/// binding or section header — blank separator lines are skipped while scrolling.
fn is_selectable(index: usize) -> bool {
    KEYBINDINGS
        .get(index)
        .is_some_and(|(key, _)| !key.is_empty())
}

/// Next selectable line below `current`, or `current` if already at the bottom.
pub fn next_selectable(current: usize) -> usize {
    ((current + 1)..KEYBINDINGS.len())
        .find(|&i| is_selectable(i))
        .unwrap_or(current)
}

/// Previous selectable line above `current`, or `current` if already at the top.
pub fn prev_selectable(current: usize) -> usize {
    (0..current)
        .rev()
        .find(|&i| is_selectable(i))
        .unwrap_or(current)
}

pub fn render(frame: &mut Frame, area: Rect, selection: usize) {
    let popup_area = centered_rect(60, 80, area);

    let block = Block::default()
        .title(Span::styled(
            " Help  [↑/↓ scroll, Esc close] ",
            Style::default().fg(theme::text()),
        ))
        .borders(Borders::ALL)
        .style(Style::default().fg(theme::popup_border()));

    let items: Vec<ListItem> = KEYBINDINGS
        .iter()
        .map(|(key, desc)| {
            if key.is_empty() {
                ListItem::new(Line::raw(""))
            } else if desc.is_empty() {
                // Section header
                ListItem::new(Line::from(Span::styled(
                    key.to_string(),
                    Style::default().fg(theme::active_border()),
                )))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {key:<20}"), Style::default().fg(theme::dir_fg())),
                    Span::styled(desc.to_string(), Style::default().fg(theme::text())),
                ]))
            }
        })
        .collect();

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(theme::highlight_bg())
            .fg(theme::highlight_fg())
            .add_modifier(Modifier::BOLD),
    );

    let mut state = ListState::default();
    state.select(Some(selection.min(KEYBINDINGS.len().saturating_sub(1))));

    frame.render_widget(Clear, popup_area);
    frame.render_stateful_widget(list, popup_area, &mut state);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let margin_v = (100 - percent_y) / 2;
    let margin_h = (100 - percent_x) / 2;

    let vertical = Layout::vertical([
        Constraint::Percentage(margin_v),
        Constraint::Percentage(percent_y),
        Constraint::Percentage(margin_v),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage(margin_h),
        Constraint::Percentage(percent_x),
        Constraint::Percentage(margin_h),
    ])
    .split(vertical[1])[1]
}
