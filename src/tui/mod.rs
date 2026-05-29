pub mod app;
pub mod events;
pub mod ui;

use app::App;
use crossterm::{execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use crate::config::Config;
use ratatui::{backend::CrosstermBackend, Terminal};

pub fn run(cfg: &Config) {
    enable_raw_mode().expect("enable raw mode");
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout)).unwrap();

    let mut app = App::new(cfg);
    loop {
        terminal.draw(|f| ui::draw(f, &app)).unwrap();
        events::handle(&mut app).unwrap();
        if app.quit { break; }
    }

    disable_raw_mode().unwrap();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).unwrap();
    terminal.show_cursor().unwrap();
}
