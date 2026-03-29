use std::io;

use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Direction, Layout},
};

mod app;
mod ui;

use app::{ActivePanel, App};

fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal) -> io::Result<()> {
    let mut state = App::new()?;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let vertical = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(area);

            let top = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Min(0)])
                .split(vertical[0]);

            ui::dir_panel::render(
                frame,
                top[0],
                state.dirs(),
                state.active_panel == ActivePanel::Dirs,
                state.dir_selection,
            );
            ui::file_panel::render(
                frame,
                top[1],
                state.files(),
                state.active_panel == ActivePanel::Files,
                state.file_selection,
            );
            ui::command_prompt::render(
                frame,
                vertical[1],
                state.active_panel == ActivePanel::Command,
            );
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Tab => state.active_panel = state.active_panel.next(),
                KeyCode::BackTab => state.active_panel = state.active_panel.prev(),
                KeyCode::Up => state.select_up(),
                KeyCode::Down => state.select_down(),
                _ => {}
            }
        }
    }
}
