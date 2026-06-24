mod app;
mod reader;
mod state;
mod ui;

use anyhow::Context;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use std::io;

#[derive(Parser)]
#[command(
    name = "eread",
    about = "A TUI EPUB reader that remembers where you left off"
)]
struct Cli {
    /// Path to the EPUB file to open
    #[arg()]
    epub: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Prepare the terminal.
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)
        .context("failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        ratatui::Terminal::new(backend).context("failed to create terminal")?;

    // Open the EPUB and restore any prior reading position.
    let mut app = app::App::open(&cli.epub)?;

    // Main event loop.
    let res = run_app(&mut terminal, &mut app);

    // Put the terminal back the way we found it.
    disable_raw_mode().ok();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    terminal.show_cursor().ok();

    // Remember where we stopped.
    if let Err(e) = app.save_state() {
        eprintln!("Warning: failed to save reading position: {e}");
    }

    res
}

fn run_app(
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut app::App,
) -> anyhow::Result<()> {
    loop {
        terminal
            .draw(|frame| ui::draw(frame, app))
            .context("terminal draw failed")?;

        match event::read().context("failed to read input event")? {
            Event::Key(key)
                if key.kind != KeyEventKind::Release
                    && app.handle_key(key.code) =>
            {
                return Ok(());
            }
            _ => {}
        }
    }
}
