use std::path::Path;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::model::TransferMode;
use crate::theme;

/// Rows the bar takes off the top of the destination panel.
pub const HEIGHT: u16 = 3;

/// The banner above the destination panel while a copy or move picks its
/// target. It spells out the folder the transfer will land in, so the
/// destination is readable even when the panel title is cut short.
pub fn render(frame: &mut Frame, area: Rect, mode: TransferMode, target: &Path) {
    let (label, color) = if mode.is_move() {
        (" move to ", theme::move_target_border())
    } else {
        (" copy to ", theme::copy_target_border())
    };

    // Two columns go to the borders, two more to the padding around the path.
    let inner = area.width.saturating_sub(4) as usize;
    let path = fit(&target.display().to_string(), inner);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(Span::styled(label, Style::default().fg(color)));

    let text = Line::from(Span::styled(
        path,
        Style::default()
            .fg(theme::text())
            .add_modifier(Modifier::BOLD),
    ));

    frame.render_widget(Paragraph::new(text).block(block), area);
}

/// Shorten a path to `width` columns by dropping its head: the folder the
/// transfer lands in is the tail, so that is the part worth keeping.
fn fit(path: &str, width: usize) -> String {
    let len = path.chars().count();
    if len <= width {
        return path.to_owned();
    }
    if width <= 1 {
        return "\u{2026}".repeat(width);
    }
    let skip = len - (width - 1);
    let tail: String = path.chars().skip(skip).collect();
    format!("\u{2026}{tail}")
}

#[cfg(test)]
mod tests {
    use super::fit;

    #[test]
    fn a_path_that_fits_is_left_alone() {
        assert_eq!(fit("/tmp/a", 10), "/tmp/a");
    }

    #[test]
    fn a_long_path_keeps_its_tail() {
        assert_eq!(fit("/tmp/one/two/three", 8), "\u{2026}o/three");
    }

    #[test]
    fn a_bar_with_no_room_renders_nothing() {
        assert_eq!(fit("/tmp/a", 0), "");
    }
}
