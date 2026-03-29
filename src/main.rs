use std::io;

use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Direction, Layout},
};

mod ui;

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

            ui::file_panel::render(frame, chunks[0]);
            ui::command_prompt::render(frame, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()?
            && key.code == KeyCode::Char('q')
        {
            return Ok(());
        }
    }
}
