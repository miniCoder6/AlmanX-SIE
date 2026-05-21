// ─── tui/events.rs ────────────────────────────────────────────────────────────
//
// Keyboard event dispatch.  Each mode has its own handler so the logic stays
// readable as modes grow.

use super::app::{App, Mode};
use crate::ops::alias_file::{add_alias_to_files, remove_alias_from_files};
use crossterm::event::{KeyCode, KeyModifiers};

pub fn handle(app: &mut App, key: KeyCode, mods: KeyModifiers) {
    // Ctrl-C always quits.
    if key == KeyCode::Char('c') && mods.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return;
    }

    match app.mode.clone() {
        Mode::Browse           => handle_browse(app, key),
        Mode::Search           => handle_search(app, key),
        Mode::AddAlias { command }           => handle_add_alias(app, key, command),
        Mode::ConfirmAlias { command, alias } => handle_confirm(app, key, command, alias),
        Mode::ListAliases      => handle_list(app, key),
        Mode::Popup { .. }     => { app.mode = Mode::Browse; } // any key closes
    }
}

// ── Browse ────────────────────────────────────────────────────────────────────

fn handle_browse(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,

        KeyCode::Char('/') => {
            app.mode = Mode::Search;
        }

        KeyCode::Down  | KeyCode::Char('j') => app.scroll_down(),
        KeyCode::Up    | KeyCode::Char('k') => app.scroll_up(),

        KeyCode::Char('a') | KeyCode::Enter => {
            if let Some(cmd) = app.selected_command().cloned() {
                app.alias_input.clear();
                app.load_suggestions_for(&cmd.text);
                app.mode = Mode::AddAlias { command: cmd.text };
            }
        }

        KeyCode::Char('d') => {
            if let Some(cmd) = app.selected_command().cloned() {
                app.db.tombstone(&cmd.text, &mut app.deleted);
                app.reload_commands();
                app.set_popup(format!("Dismissed: {}", cmd.text));
            }
        }

        KeyCode::Char('l') => {
            app.load_aliases();
            app.mode = Mode::ListAliases;
        }

        _ => {}
    }
}

// ── Search ────────────────────────────────────────────────────────────────────

fn handle_search(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Enter => {
            app.mode = Mode::Browse;
        }
        KeyCode::Backspace => {
            app.search.pop();
            app.apply_search();
        }
        KeyCode::Char(c) => {
            app.search.push(c);
            app.apply_search();
        }
        _ => {}
    }
}

// ── Add Alias ─────────────────────────────────────────────────────────────────

fn handle_add_alias(app: &mut App, key: KeyCode, command: String) {
    match key {
        KeyCode::Esc => {
            app.mode = Mode::Browse;
        }

        KeyCode::Down | KeyCode::Char('j') => {
            let len = app.alias_suggestions.len();
            if len > 0 {
                let next = app.suggestions_state.selected()
                    .unwrap_or(0).saturating_add(1).min(len - 1);
                app.suggestions_state.select(Some(next));
                // Fill input from suggestion.
                if let Some(s) = app.alias_suggestions.get(next) {
                    app.alias_input = s.alias.clone();
                }
            }
        }

        KeyCode::Up | KeyCode::Char('k') => {
            if !app.alias_suggestions.is_empty() {
                let prev = app.suggestions_state.selected()
                    .unwrap_or(0).saturating_sub(1);
                app.suggestions_state.select(Some(prev));
                if let Some(s) = app.alias_suggestions.get(prev) {
                    app.alias_input = s.alias.clone();
                }
            }
        }

        KeyCode::Tab => {
            // Accept highlighted suggestion into input.
            if let Some(i) = app.suggestions_state.selected() {
                if let Some(s) = app.alias_suggestions.get(i) {
                    app.alias_input = s.alias.clone();
                }
            }
        }

        KeyCode::Enter => {
            let alias = app.alias_input.trim().to_owned();
            if alias.is_empty() {
                app.set_popup("Alias name cannot be empty.".to_owned());
            } else {
                app.mode = Mode::ConfirmAlias { command, alias };
            }
        }

        KeyCode::Backspace => {
            app.alias_input.pop();
        }

        KeyCode::Char(c) => {
            app.alias_input.push(c);
        }

        _ => {}
    }
}

// ── Confirm Alias ─────────────────────────────────────────────────────────────

fn handle_confirm(app: &mut App, key: KeyCode, command: String, alias: String) {
    match key {
        KeyCode::Enter | KeyCode::Char('y') => {
            add_alias_to_files(&app.alias_files, &alias, &command);
            app.reload_commands();
            app.set_popup(format!("Added: alias {}='{}'", alias, command));
        }
        KeyCode::Esc | KeyCode::Char('n') => {
            app.mode = Mode::Browse;
        }
        _ => {}
    }
}

// ── List Aliases ──────────────────────────────────────────────────────────────

fn handle_list(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Browse;
        }
        KeyCode::Down | KeyCode::Char('j') => app.scroll_aliases_down(),
        KeyCode::Up   | KeyCode::Char('k') => app.scroll_aliases_up(),

        KeyCode::Char('d') => {
            if let Some(i) = app.aliases_state.selected() {
                if let Some((name, _)) = app.aliases.get(i).cloned() {
                    remove_alias_from_files(&app.alias_files, &name);
                    app.load_aliases();
                    app.set_popup(format!("Removed alias: {}", name));
                }
            }
        }
        _ => {}
    }
}
