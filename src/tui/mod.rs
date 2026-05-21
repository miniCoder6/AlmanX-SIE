// ─── tui/mod.rs ───────────────────────────────────────────────────────────────
pub mod app;
pub mod events;
pub mod render;

use crate::database::persistence;
use app::App;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub fn run(alias_file: String, alias_files: Vec<String>) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let db      = persistence::load_db();
    let deleted = persistence::load_deleted();
    let mut app = App::new(db, deleted, alias_file, alias_files);
    app.reload_commands();

    let result = event_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| render::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            events::handle(app, key.code, key.modifiers);
        }

        if app.should_quit {
            let _ = persistence::save_db(&app.db);
            let _ = persistence::save_deleted(&app.deleted);
            break;
        }
    }
    Ok(())
}
