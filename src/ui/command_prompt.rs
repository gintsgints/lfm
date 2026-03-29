use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

pub fn render(frame: &mut Frame, area: Rect) {
    let widget = Paragraph::new(Text::raw("Press 'q' to quit")).block(
        Block::default()
            .title(" Command ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(widget, area);
}
