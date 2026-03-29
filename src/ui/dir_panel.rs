use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

pub fn render(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Directories ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White));

    frame.render_widget(Paragraph::new(Text::raw("")).block(block), area);
}
