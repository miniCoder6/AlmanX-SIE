// ─── tui/app.rs ───────────────────────────────────────────────────────────────
use crate::database::{Database, DeletedCommands, structs::Command};
use crate::ops::{suggest::AliasSuggestion, AliasSuggester};
use almanx_workflow_engine::{WorkflowDag, WorkflowEngine};
use almanx_storage::EventLog;
use ratatui::widgets::ListState;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Browse,
    Search,
    AddAlias { command: String },
    ConfirmAlias { command: String, alias: String },
    ListAliases,
    Workflows,
    Popup { message: String },
}

pub struct App {
    pub db:          Database,
    pub deleted:     DeletedCommands,
    pub alias_file:  String,
    pub alias_files: Vec<String>,

    pub mode:        Mode,
    pub should_quit: bool,

    pub commands:    Vec<Command>,
    pub filtered:    Vec<Command>,
    pub list_state:  ListState,

    pub search:      String,

    pub alias_input:       String,
    pub alias_suggestions: Vec<AliasSuggestion>,
    pub suggestions_state: ListState,

    pub aliases:         Vec<(String, String)>,
    pub aliases_state:   ListState,

    pub workflows:       Vec<WorkflowDag>,
    pub workflows_state: ListState,

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

        App {
            db, deleted, alias_file, alias_files,
            mode: Mode::Browse,
            should_quit: false,
            commands: vec![],
            filtered: vec![],
            list_state,
            search: String::new(),
            alias_input: String::new(),
            alias_suggestions: vec![],
            suggestions_state: { let mut s = ListState::default(); s.select(Some(0)); s },
            aliases: vec![],
            aliases_state: { let mut s = ListState::default(); s.select(Some(0)); s },
            workflows: vec![],
            workflows_state: { let mut s = ListState::default(); s.select(Some(0)); s },
            status: " q=quit  /=search  a=alias  l=list  w=workflows  d=dismiss".to_owned(),
        }
    }

    pub fn reload_commands(&mut self) {
        self.commands = self.db.top(100).into_iter().cloned().collect();
        self.apply_search();
    }

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

    pub fn scroll_workflows_down(&mut self) {
        let len = self.workflows.len();
        if len == 0 { return; }
        let next = self.workflows_state.selected().unwrap_or(0).saturating_add(1).min(len - 1);
        self.workflows_state.select(Some(next));
    }

    pub fn scroll_workflows_up(&mut self) {
        let prev = self.workflows_state.selected().unwrap_or(0).saturating_sub(1);
        self.workflows_state.select(Some(prev));
    }

    pub fn load_suggestions_for(&mut self, command: &str) {
        let suggester = AliasSuggester::new(&self.alias_file);
        self.alias_suggestions = suggester.suggest(command);
        let len = self.alias_suggestions.len();
        self.suggestions_state.select(if len > 0 { Some(0) } else { None });
    }

    pub fn load_aliases(&mut self) {
        self.aliases = crate::ops::alias_file::get_all_aliases(&self.alias_files);
        self.aliases_state.select(if self.aliases.is_empty() { None } else { Some(0) });
    }

    pub fn load_workflows(&mut self) {
        let log_path = crate::database::persistence::event_log_path();
        let log = EventLog::new(log_path);
        if let Ok(events) = log.read_all() {
            let engine = WorkflowEngine::new(30 * 60 * 1000); // 30 min session
            self.workflows = engine.mine_workflows(&events);
            self.workflows = self.workflows.iter()
                .filter(|w| w.frequency >= 2)
                .cloned()
                .collect();
        }
        self.workflows_state.select(if self.workflows.is_empty() { None } else { Some(0) });
    }

    pub fn set_popup(&mut self, msg: impl Into<String>) {
        self.mode = Mode::Popup { message: msg.into() };
    }
}
