use std::io;

use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal) -> io::Result<()> {
    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(area);

            let file_panel = Block::default()
                .title(" Files ")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White));

            let placeholder = Paragraph::new(Text::raw("")).block(file_panel);

            frame.render_widget(placeholder, chunks[0]);

            let command_prompt = Paragraph::new(Text::raw("Press 'q' to quit")).block(
                Block::default()
                    .title(" Command ")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Yellow)),
            );

            frame.render_widget(command_prompt, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()?
            && key.code == KeyCode::Char('q')
        {
            return Ok(());
        }
    }
}
