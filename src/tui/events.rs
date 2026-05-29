use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;
use crate::tui::app::{Action, App, Mode};

pub fn handle(app: &mut App) -> Result<()> {
    if !event::poll(Duration::from_millis(50))? { return Ok(()); }
    if let Event::Key(k) = event::read()? {
        if k.kind != event::KeyEventKind::Press { return Ok(()); }
        match app.mode.clone() {
            Mode::Main           => main_keys(app, k.code),
            Mode::Search         => search_keys(app, k.code),
            Mode::AddAlias       => add_alias_keys(app, k.code),
            Mode::PickSuggestion => pick_sug_keys(app, k.code),
            Mode::RemoveAlias    => text_input_keys(app, k.code, |a| {
                if !a.input.is_empty() {
                    a.confirm_yes = true;
                    a.mode = Mode::Confirm(Action::RemoveAlias { alias: a.input.clone() });
                }
            }),
            Mode::ChangeAlias    => change_alias_keys(app, k.code),
            Mode::ListAliases    => list_keys(app, k.code),
            Mode::Stats | Mode::Query | Mode::Predict | Mode::Context => output_keys(app, k.code),
            Mode::Confirm(_)     => confirm_keys(app, k.code),
        }
    }
    Ok(())
}

fn main_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q')             => app.quit = true,
        KeyCode::Char('s') | KeyCode::Char('/') => { app.search_query.clear(); app.mode = Mode::Search; }
        KeyCode::Char('a') => {
            app.alias_input.clear(); app.input.clear();
            if let Some(rec) = app.selected_command() {
                app.input = rec.command.clone();
                let cmd = app.input.clone();
                app.load_suggestions(&cmd);
            }
            app.add_alias_focus_command = false;
            app.mode = Mode::AddAlias;
        }
        KeyCode::Char('r') => { app.input.clear(); app.mode = Mode::RemoveAlias; }
        KeyCode::Char('c') => { app.input.clear(); app.mode = Mode::ChangeAlias; }
        KeyCode::Char('l') => { app.reload_aliases(); app.mode = Mode::ListAliases; }
        KeyCode::Char('t') => { app.compute_stats(); app.mode = Mode::Stats; }
        KeyCode::Char('p') => { app.run_predict(); app.mode = Mode::Predict; }
        KeyCode::Char('x') => { app.run_context(); app.mode = Mode::Context; }
        KeyCode::Char('Q') => { app.query_input.clear(); app.output_lines.clear(); app.mode = Mode::Query; }
        KeyCode::Down | KeyCode::Char('j') => { let n = app.visible.len(); app.nav_down(n); }
        KeyCode::Up   | KeyCode::Char('k') => app.nav_up(),
        KeyCode::F(5)                      => app.reload(),
        KeyCode::Esc                       => app.filter_clear(),
        KeyCode::Backspace                 => app.filter_pop(),
        KeyCode::Char(c)                   => app.filter_push(c),
        _ => {}
    }
}

fn search_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc       => app.mode = Mode::Main,
        KeyCode::Backspace => { app.search_query.pop(); app.run_search(); }
        KeyCode::Char(c)   => { app.search_query.push(c); app.run_search(); }
        KeyCode::Down      => { let n = app.search_results.len(); app.search_state.select(app.search_state.selected().map(|i| (i+1).min(n.saturating_sub(1)))); }
        KeyCode::Up        => { app.search_state.select(app.search_state.selected().map(|i| i.saturating_sub(1))); }
        _ => {}
    }
}

fn add_alias_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc       => app.mode = Mode::Main,
        KeyCode::Tab       => { if !app.suggestions.is_empty() { app.mode = Mode::PickSuggestion; } }
        KeyCode::Up | KeyCode::Down => { app.add_alias_focus_command = !app.add_alias_focus_command; }
        KeyCode::Backspace => {
            if app.add_alias_focus_command { app.input.pop(); } else { app.alias_input.pop(); }
        }
        KeyCode::Char(c)   => {
            if app.add_alias_focus_command { app.input.push(c); } else { app.alias_input.push(c); }
        }
        KeyCode::Enter => {
            if app.input.is_empty() || app.alias_input.is_empty() {
                app.status = "Fill in both command and alias.".into(); return;
            }
            app.confirm_yes = true;
            app.mode = Mode::Confirm(Action::AddAlias { alias: app.alias_input.clone(), command: app.input.clone() });
        }
        _ => {}
    }
}

fn pick_sug_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc   => app.mode = Mode::AddAlias,
        KeyCode::Down  => { let n = app.suggestions.len(); app.sug_state.select(app.sug_state.selected().map(|i| (i+1).min(n.saturating_sub(1)))); }
        KeyCode::Up    => { app.sug_state.select(app.sug_state.selected().map(|i| i.saturating_sub(1))); }
        KeyCode::Enter => {
            if let Some(i) = app.sug_state.selected() {
                if let Some(s) = app.suggestions.get(i) { app.alias_input = s.alias.clone(); }
            }
            app.mode = Mode::AddAlias;
        }
        _ => {}
    }
}

fn text_input_keys(app: &mut App, code: KeyCode, on_enter: impl FnOnce(&mut App)) {
    match code {
        KeyCode::Esc       => app.mode = Mode::Main,
        KeyCode::Backspace => { app.input.pop(); }
        KeyCode::Char(c)   => app.input.push(c),
        KeyCode::Enter     => on_enter(app),
        _ => {}
    }
}

fn change_alias_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc       => app.mode = Mode::Main,
        KeyCode::Backspace => { app.input.pop(); }
        KeyCode::Char(c)   => app.input.push(c),
        KeyCode::Enter => {
            let parts: Vec<String> = app.input.splitn(3, ' ').map(|s| s.to_string()).collect();
            if parts.len() == 3 {
                app.alias_store.change(&parts[0], &parts[1], &parts[2]);
                app.reload_aliases();
                app.status = format!("Changed {} → {}", parts[0], parts[1]);
                app.mode = Mode::Main;
            } else {
                app.status = "Format: old_alias new_alias command".into();
            }
        }
        _ => {}
    }
}

fn list_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => app.mode = Mode::Main,
        KeyCode::Down => { let n = app.aliases.len(); app.alias_state.select(app.alias_state.selected().map(|i| (i+1).min(n.saturating_sub(1)))); }
        KeyCode::Up   => { app.alias_state.select(app.alias_state.selected().map(|i| i.saturating_sub(1))); }
        _ => {}
    }
}

fn output_keys(app: &mut App, code: KeyCode) {
    match &app.mode.clone() {
        Mode::Query => match code {
            KeyCode::Esc       => app.mode = Mode::Main,
            KeyCode::Enter     => app.run_query(),
            KeyCode::Backspace => { app.query_input.pop(); }
            KeyCode::Char(c)   => app.query_input.push(c),
            _ => {}
        },
        _ => { if matches!(code, KeyCode::Esc | KeyCode::Char('q')) { app.mode = Mode::Main; } }
    }
}

fn confirm_keys(app: &mut App, code: KeyCode) {
    let action = match app.mode.clone() { Mode::Confirm(a) => a, _ => return };
    match code {
        KeyCode::Left | KeyCode::Right => app.confirm_yes = !app.confirm_yes,
        KeyCode::Esc   => app.mode = Mode::Main,
        KeyCode::Enter => {
            if app.confirm_yes {
                match &action {
                    Action::AddAlias { alias, command } => {
                        app.alias_store.add(alias, command);
                        app.status = format!("Added: {} = '{}'", alias, command);
                        app.reload_aliases();
                    }
                    Action::RemoveAlias { alias } => {
                        app.alias_store.remove(alias);
                        app.status = format!("Removed: {}", alias);
                        app.reload_aliases();
                    }
                }
            }
            app.mode = Mode::Main;
        }
        _ => {}
    }
}
