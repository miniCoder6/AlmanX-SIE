mod main_view;
mod search_view;
mod add_alias_view;
mod pick_suggestion_view;
mod remove_alias_view;
mod change_alias_view;
mod list_aliases_view;
mod output_view;
mod confirm_view;

use crate::tui::app::{App, Mode};
use ratatui::Frame;

const ACCENT: ratatui::style::Color = ratatui::style::Color::Rgb(56, 139, 253);
const GOLD:   ratatui::style::Color = ratatui::style::Color::Rgb(210, 153, 34);
const DIM:    ratatui::style::Color = ratatui::style::Color::Rgb(110, 110, 120);
const GREEN:  ratatui::style::Color = ratatui::style::Color::Rgb(80, 200, 100);

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.size();
    match &app.mode {
        Mode::Main           => main_view::render(f, app, area),
        Mode::Search         => search_view::render(f, app, area),
        Mode::AddAlias       => add_alias_view::render(f, app, area),
        Mode::PickSuggestion => pick_suggestion_view::render(f, app, area),
        Mode::RemoveAlias    => remove_alias_view::render(f, app, area),
        Mode::ChangeAlias    => change_alias_view::render(f, app, area),
        Mode::ListAliases    => list_aliases_view::render(f, app, area),
        Mode::Stats          => output_view::render(f, app, area, "Workflow Stats"),
        Mode::Query          => output_view::render_query(f, app, area),
        Mode::Confirm(_)     => { main_view::render(f, app, area); confirm_view::render(f, app, area); }
    }
}

fn block(title: &str) -> ratatui::widgets::Block {
    use ratatui::{style::{Modifier, Style}, text::Span, widgets::{Block, BorderType, Borders}};
    Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
        .title(Span::styled(format!(" {} ", title), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
}

fn status(msg: &str) -> ratatui::widgets::Paragraph {
    use ratatui::{style::{Color, Style}, widgets::Paragraph};
    Paragraph::new(format!(" {}", msg)).style(Style::default().fg(Color::Black).bg(ACCENT))
}
