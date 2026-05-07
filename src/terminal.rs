use std::{
    io::{self, Stdout},
    time::Duration,
};

use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{events::handle_events, model::App, ui::draw};

pub(crate) type Tui = Terminal<CrosstermBackend<Stdout>>;

pub(crate) fn setup_terminal() -> io::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

pub(crate) fn restore_terminal(terminal: &mut Tui) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()
}

pub(crate) fn run_app(terminal: &mut Tui, app: &mut App) -> io::Result<()> {
    terminal.draw(|frame| draw(frame, app))?;
    while app.running {
        let events = read_event_batch()?;
        let changed = handle_events(app, events);
        if changed {
            terminal.draw(|frame| draw(frame, app))?;
        }
    }
    Ok(())
}

pub(crate) fn read_event_batch() -> io::Result<Vec<Event>> {
    let mut events = vec![event::read()?];
    while events.len() < 4096 && event::poll(Duration::from_millis(8))? {
        events.push(event::read()?);
    }
    Ok(events)
}
