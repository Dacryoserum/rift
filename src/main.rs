use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use rift::app::App;
use rift::config::Config;
use rift::event::AppEvent;

#[derive(Parser)]
#[command(
    name = "rift",
    about = "Navigate the depths of massive text files",
    version
)]
struct Cli {
    /// File to open
    file: PathBuf,

    /// Jump to line on open
    #[arg(short = 'n', long)]
    line: Option<u64>,

    /// Jump to byte offset on open
    #[arg(short = 'b', long)]
    byte: Option<u64>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Arc::new(Config::load().unwrap_or_default());

    // Restore terminal on panic so the shell isn't left broken.
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        orig_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, cli, config);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
    Ok(())
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    cli: Cli,
    config: Arc<Config>,
) -> Result<()> {
    let (mut app, rx) = App::new(cli.file, config)?;

    // Apply initial jump
    if let Some(line) = cli.line {
        app.scroll_to_line(line.saturating_sub(1));
    }
    if let Some(byte) = cli.byte {
        let line_num = {
            let index = app.line_index.read().unwrap();
            index.line_at_offset(byte)
        };
        app.scroll_to_line(line_num);
    }

    loop {
        terminal.draw(|f| rift::ui::render(f, &app))?;

        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => app.handle_event(event)?,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                app.handle_event(AppEvent::Tick)?;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }

        if app.should_quit {
            app.bookmarks.save().ok();
            break;
        }
    }

    Ok(())
}
