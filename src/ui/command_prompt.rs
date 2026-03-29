use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

pub fn render(frame: &mut Frame, area: Rect, active: bool) {
    let border_style = if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let widget = Paragraph::new(Text::raw("Press 'q' to quit")).block(
        Block::default()
            .title(" Command ")
            .borders(Borders::ALL)
            .style(border_style),
    );
    frame.render_widget(widget, area);
}
