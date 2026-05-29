use crate::config::Config;
use crate::search::{AliasSuggester, Suggestion as Suggestion, SearchEngine, SearchResult};
use crate::store::{Aliases, Record, Store, Wal};
use ratatui::widgets::ListState;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Main,
    Search,
    AddAlias,
    PickSuggestion,
    RemoveAlias,
    ChangeAlias,
    ListAliases,
    Stats,
    Query,
    Confirm(Action),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    AddAlias    { alias: String, command: String },
    RemoveAlias { alias: String },
}

pub struct App {
    pub cfg: Config,
    pub mode: Mode,
    pub quit: bool,
    pub status: String,

    // Main list
    pub commands: Vec<Record>,
    pub list_state: ListState,
    pub filter: String,
    pub visible: Vec<usize>,     // indices into `commands` passing the filter

    // Search
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub search_state: ListState,

    // Aliases
    pub alias_store: Aliases,
    pub aliases: Vec<(String, String)>,
    pub alias_state: ListState,

    // Inputs
    pub input: String,       // command field in add/remove/change
    pub alias_input: String, // alias field in add

    // Suggestions
    pub suggestions: Vec<Suggestion>,
    pub sug_state: ListState,

    // Confirm dialog
    pub confirm_yes: bool,

    // Stats / query output
    pub output_lines: Vec<String>,
    pub query_input: String,
}

impl App {
    pub fn new(cfg: &Config) -> Self {
        let alias_store = Aliases::new(cfg.alias_file_paths.clone());
        let aliases = alias_store.all();
        let commands = load_commands(cfg);
        let visible = (0..commands.len()).collect();
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        App {
            cfg: cfg.clone(), mode: Mode::Main, quit: false,
            status: "flux  •  type to filter  •  ? for help  •  q quit".into(),
            commands, list_state, filter: String::new(), visible,
            search_query: String::new(), search_results: vec![], search_state: ListState::default(),
            alias_store, aliases, alias_state: ListState::default(),
            input: String::new(), alias_input: String::new(),
            suggestions: vec![], sug_state: ListState::default(),
            confirm_yes: true,
            output_lines: vec![], query_input: String::new(),
        }
    }

    // ── filter ────────────────────────────────────────────────────────────────

    pub fn apply_filter(&mut self) {
        let q = self.filter.to_lowercase();
        self.visible = self.commands.iter().enumerate()
            .filter(|(_, r)| q.is_empty() || r.command.to_lowercase().contains(&q))
            .map(|(i, _)| i).collect();
        self.list_state.select(if self.visible.is_empty() { None } else { Some(0) });
    }

    pub fn filter_push(&mut self, c: char) { self.filter.push(c); self.apply_filter(); }
    pub fn filter_pop(&mut self)           { self.filter.pop();    self.apply_filter(); }
    pub fn filter_clear(&mut self)         { self.filter.clear();  self.apply_filter(); }

    // ── navigation ─────────────────────────────────────────────────────────────

    pub fn nav_down(&mut self, len: usize) {
        if len == 0 { return; }
        let i = self.list_state.selected().unwrap_or(0);
        self.list_state.select(Some((i + 1).min(len - 1)));
    }
    pub fn nav_up(&mut self) {
        let i = self.list_state.selected().unwrap_or(0);
        self.list_state.select(Some(i.saturating_sub(1)));
    }

    // ── accessors ─────────────────────────────────────────────────────────────

    pub fn selected_command(&self) -> Option<&Record> {
        let vis = self.list_state.selected()?;
        self.commands.get(*self.visible.get(vis)?)
    }

    pub fn reload(&mut self) {
        self.commands = load_commands(&self.cfg);
        self.apply_filter();
    }

    pub fn reload_aliases(&mut self) {
        self.aliases = self.alias_store.all();
    }

    pub fn load_suggestions(&mut self, cmd: &str) {
        let existing: Vec<String> = self.aliases.iter().map(|(a,_)| a.clone()).collect();
        self.suggestions = AliasSuggester::new(existing).suggest(cmd);
        self.sug_state = ListState::default();
        self.sug_state.select(if self.suggestions.is_empty() { None } else { Some(0) });
    }

    pub fn run_search(&mut self) {
        let mut eng = SearchEngine::new(&self.cfg);
        for r in &self.commands { eng.index(r); }
        self.search_results = eng.search(&self.search_query, 50);
        self.search_state = ListState::default();
        self.search_state.select(if self.search_results.is_empty() { None } else { Some(0) });
    }

    pub fn compute_stats(&mut self) {
        let aliased: Vec<String> = self.aliases.iter().map(|(_, c)| c.clone()).collect();
        let refs: Vec<&Record> = self.commands.iter().collect();
        let s = crate::miner::compute_stats(&refs, &aliased);
        self.output_lines = vec![
            format!("Total commands    {}", s.total_commands),
            format!("Unique commands   {}", s.unique_commands),
            format!("Total keystrokes  {}", s.total_keystrokes),
            format!("Keystroke savings {}", s.potential_savings),
            String::new(),
            "── Top alias candidates ─────────────────────────────────".into(),
        ];
        for (cmd, freq, saving) in &s.top_candidates {
            self.output_lines.push(format!("  ×{:<4} {}  ({} keystrokes)", freq, cmd, saving));
        }
    }

    pub fn run_query(&mut self) {
        let refs: Vec<&Record> = self.commands.iter().collect();
        self.output_lines = match crate::query::parse(&self.query_input) {
            Err(e) => vec![format!("Error: {}", e)],
            Ok(q) => {
                let rows = crate::query::execute(&q, &refs);
                if rows.is_empty() { vec!["No results.".into()] }
                else { rows.iter().map(|r| format!("  {} (freq={}, score={})", r.command, r.frequency, r.score)).collect() }
            }
        };
    }
}

fn load_commands(cfg: &Config) -> Vec<Record> {
    let mut store = Store::load(&cfg.store_path());
    if store.index.is_empty() {
        if let Some(wal) = Wal::open(&cfg.wal_path(), cfg.max_wal_events) {
            wal.replay(|ev| store.ingest(&ev));
        }
    }
    store.all_sorted().into_iter().cloned().collect()
}
