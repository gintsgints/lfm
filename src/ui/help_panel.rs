use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
};

use crate::theme;

const KEYBINDINGS: &[(&str, &str)] = &[
    ("Navigation", ""),
    ("j / ↓", "Move down"),
    ("k / ↑", "Move up"),
    ("h / ←", "Go to parent directory"),
    ("l / →", "Enter directory"),
    ("Tab", "Next panel"),
    ("Shift+Tab", "Previous panel"),
    ("", ""),
    ("Selection", ""),
    ("J / Shift+↓", "Mark item and move down"),
    ("K / Shift+↑", "Mark item and move up"),
    ("Esc", "Clear selection"),
    ("", ""),
    ("File operations", ""),
    ("n", "Create file or directory"),
    ("r", "Rename current item"),
    ("g", "Go to path"),
    ("d", "Delete selected or current item"),
    ("c", "Copy selected or current item"),
    ("C", "Copy single item — rename before placing"),
    ("m", "Move selected or current item"),
    ("M", "Move single item — rename before placing"),
    ("e", "Open selected item in $EDITOR"),
    ("o", "Open with default application"),
    ("s", "Cycle sort: name / date / ext / size"),
    ("z", "Zip selected or current item(s)"),
    ("u", "Extract .zip or .tar.gz archive"),
    ("", ""),
    ("Filter", ""),
    ("/", "Enter filter mode"),
    ("Enter / Esc", "Exit filter, restore path and selection"),
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

pub fn render(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 80, area);

    let block = Block::default()
        .title(Span::styled(
            " Help  [Esc] close ",
            Style::default().fg(theme::TEXT),
        ))
        .borders(Borders::ALL)
        .style(Style::default().fg(theme::POPUP_BORDER));

    let items: Vec<ListItem> = KEYBINDINGS
        .iter()
        .map(|(key, desc)| {
            if key.is_empty() {
                ListItem::new(Line::raw(""))
            } else if desc.is_empty() {
                // Section header
                ListItem::new(Line::from(Span::styled(
                    key.to_string(),
                    Style::default().fg(theme::ACTIVE_BORDER),
                )))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {key:<20}"), Style::default().fg(theme::DIR_FG)),
                    Span::styled(desc.to_string(), Style::default().fg(theme::TEXT)),
                ]))
            }
        })
        .collect();

    let list = List::new(items).block(block);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(list, popup_area);
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
