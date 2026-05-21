// ─── tui/app.rs ───────────────────────────────────────────────────────────────
//
// All TUI state lives here.  The `App` struct is the single source of truth —
// the renderer reads from it, the event handler mutates it.

use crate::database::{Database, DeletedCommands, structs::Command};
use crate::ops::{suggest::AliasSuggestion, AliasSuggester};
use ratatui::widgets::ListState;

// ── Modes ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    /// Main view: command list + search box.
    Browse,
    /// User is typing in the search box.
    Search,
    /// User pressed Enter on a command — picking an alias name.
    AddAlias { command: String },
    /// Confirm before writing the alias.
    ConfirmAlias { command: String, alias: String },
    /// Browsing all tracked aliases.
    ListAliases,
    /// Show a status popup (success / error message).
    Popup { message: String },
}

// ── App ───────────────────────────────────────────────────────────────────────

pub struct App {
    // ── Persistence ──────────────────────────────────────────────────────────
    pub db:          Database,
    pub deleted:     DeletedCommands,
    pub alias_file:  String,
    pub alias_files: Vec<String>,

    // ── UI state ─────────────────────────────────────────────────────────────
    pub mode:        Mode,
    pub should_quit: bool,

    /// All commands loaded from DB, refreshed on mode change.
    pub commands:    Vec<Command>,
    /// Subset of `commands` matching the current search query.
    pub filtered:    Vec<Command>,
    pub list_state:  ListState,

    /// Live search input.
    pub search:      String,

    /// When adding an alias: the text the user is typing for the alias name.
    pub alias_input: String,
    pub alias_suggestions: Vec<AliasSuggestion>,
    pub suggestions_state: ListState,

    /// All tracked aliases (shown in ListAliases mode).
    pub aliases:         Vec<(String, String)>,
    pub aliases_state:   ListState,

    /// A one-line status shown at the bottom of the main view.
    pub status: String,
}

impl App {
    pub fn new(
        db: Database,
        deleted: DeletedCommands,
        alias_file: String,
        alias_files: Vec<String>,
    ) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let mut suggestions_state = ListState::default();
        suggestions_state.select(Some(0));
        let mut aliases_state = ListState::default();
        aliases_state.select(Some(0));

        App {
            db,
            deleted,
            alias_file,
            alias_files,
            mode: Mode::Browse,
            should_quit: false,
            commands: vec![],
            filtered: vec![],
            list_state,
            search: String::new(),
            alias_input: String::new(),
            alias_suggestions: vec![],
            suggestions_state,
            aliases: vec![],
            aliases_state,
            status: " q=quit  /=search  a=add-alias  l=list  d=dismiss".to_owned(),
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Reload `commands` from DB and rebuild `filtered`.
    pub fn reload_commands(&mut self) {
        self.commands = self.db.top(50).into_iter().cloned().collect();
        self.apply_search();
    }

    /// Filter `commands` by the current search string.
    pub fn apply_search(&mut self) {
        let q = self.search.to_lowercase();
        self.filtered = if q.is_empty() {
            self.commands.clone()
        } else {
            self.commands
                .iter()
                .filter(|c| c.text.to_lowercase().contains(&q))
                .cloned()
                .collect()
        };

        // Keep selection in-bounds.
        let len = self.filtered.len();
        if len == 0 {
            self.list_state.select(None);
        } else {
            let sel = self.list_state.selected().unwrap_or(0).min(len - 1);
            self.list_state.select(Some(sel));
        }
    }

    pub fn selected_command(&self) -> Option<&Command> {
        let i = self.list_state.selected()?;
        self.filtered.get(i)
    }

    pub fn scroll_down(&mut self) {
        let len = self.filtered.len();
        if len == 0 { return; }
        let next = self.list_state.selected().unwrap_or(0).saturating_add(1).min(len - 1);
        self.list_state.select(Some(next));
    }

    pub fn scroll_up(&mut self) {
        let prev = self.list_state.selected().unwrap_or(0).saturating_sub(1);
        self.list_state.select(Some(prev));
    }

    pub fn scroll_aliases_down(&mut self) {
        let len = self.aliases.len();
        if len == 0 { return; }
        let next = self.aliases_state.selected().unwrap_or(0).saturating_add(1).min(len - 1);
        self.aliases_state.select(Some(next));
    }

    pub fn scroll_aliases_up(&mut self) {
        let prev = self.aliases_state.selected().unwrap_or(0).saturating_sub(1);
        self.aliases_state.select(Some(prev));
    }

    pub fn load_suggestions_for(&mut self, command: &str) {
        let suggester = AliasSuggester::new(&self.alias_file);
        self.alias_suggestions = suggester.suggest(command);
        let len = self.alias_suggestions.len();
        if len > 0 {
            self.suggestions_state.select(Some(0));
        } else {
            self.suggestions_state.select(None);
        }
    }

    pub fn load_aliases(&mut self) {
        self.aliases = crate::ops::alias_file::get_all_aliases(&self.alias_files);
        if self.aliases.is_empty() {
            self.aliases_state.select(None);
        } else {
            self.aliases_state.select(Some(0));
        }
    }

    pub fn set_popup(&mut self, msg: impl Into<String>) {
        self.mode = Mode::Popup { message: msg.into() };
    }
}
