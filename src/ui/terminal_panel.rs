use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders},
};
use tui_term::widget::{Cursor, PseudoTerminal};

use crate::terminal::TerminalPanel;
use crate::theme;

/// Render the shell panel across the bottom of the screen. `focused` follows
/// `t` / `Ctrl+O` and picks the border colour the way the file panels do.
///
/// The pty is told the panel's size from here, because the drawn area is the
/// only place it is known. `resize` ignores a size it has already been given,
/// so an unchanged panel does not signal the child on every frame.
pub fn render(frame: &mut Frame, area: Rect, panel: &mut TerminalPanel, focused: bool) {
    let border = if focused {
        theme::active_border()
    } else {
        theme::inactive_border()
    };

    let block = Block::default()
        .title(Span::styled(
            format!(" shell — {} ", panel.cwd.display()),
            Style::default().fg(border),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border));

    let inner = block.inner(area);
    panel.resize(inner.height, inner.width);

    // A block cursor in an unfocused panel would read as the place keys go, so
    // it is only drawn while the shell actually has them.
    let cursor = Cursor::default()
        .visibility(focused)
        .style(Style::default().fg(theme::active_border()));

    frame.render_widget(
        PseudoTerminal::new(panel.screen())
            .block(block)
            .cursor(cursor),
        area,
    );
}
