use flux::cli::{Cli, Cmd};
use clap::Parser;
use colored::*;
use flux::config::Config;
use flux::search::{AliasSuggester, SearchEngine};
use flux::store::{Aliases, ShellEvent, Store, Wal};
use flux::{miner, query, shell, tui};

fn main() {
    let cfg = Config::load();
    cfg.ensure_dirs();

    let args: Vec<String> = std::env::args().collect();

    // Fast path for shell hook
    if args.get(1).map(|s| s == "custom").unwrap_or(false) {
        let cmd = args[2..].join(" ");
        if !cmd.trim().is_empty() { ingest(&cfg, &cmd); }
        return;
    }

    if args.len() == 1 { tui::run(&cfg); return; }

    let cli = Cli::parse();
    let alias_paths = cli.alias_file_path.as_ref().map(|p| vec![p.clone()]).unwrap_or(cfg.alias_file_paths.clone());
    let aliases = Aliases::new(alias_paths);

    match cli.cmd {
        None => tui::run(&cfg),

        Some(Cmd::Init { shell }) => print!("{}", shell::init_script(&shell, &cfg)),

        Some(Cmd::Add { alias, command }) => {
            aliases.add(&alias, &command);
            let mut store = load_store(&cfg);
            store.delete(&command);
            store.save(&cfg.store_path());
            println!("{}", format!("Added: {} = '{}'", alias, command).green());
        }

        Some(Cmd::Remove { alias }) => {
            aliases.remove(&alias);
            println!("{}", format!("Removed: {}", alias).yellow());
        }

        Some(Cmd::Change { old_alias, new_alias, command }) => {
            aliases.change(&old_alias, &new_alias, &command);
            println!("{}", format!("Changed: {} → {} = '{}'", old_alias, new_alias, command).green());
        }

        Some(Cmd::List) => {
            let all = aliases.all();
            if all.is_empty() { println!("{}", "No aliases.".yellow()); return; }
            let wa = all.iter().map(|(a,_)| a.len()).max().unwrap_or(5);
            println!("{}", format!("  {:<wa$}  COMMAND", "ALIAS").cyan().bold());
            println!("{}", format!("  {:-<wa$}  -------", "").dimmed());
            for (a, c) in &all { println!("  {:<wa$}  {}", a.cyan(), c); }
            println!("{}", format!("\n  {} alias(es)", all.len()).dimmed());
        }

        Some(Cmd::Suggest { num }) => {
            let store = load_store(&cfg);
            let aliased_cmds: Vec<String> = aliases.all().into_iter().map(|(_,c)| c).collect();
            let alias_names: Vec<String>  = aliases.all().into_iter().map(|(a,_)| a).collect();
            let suggester = AliasSuggester::new(alias_names);
            let top: Vec<_> = store.all_sorted().into_iter()
                .filter(|r| !aliased_cmds.contains(&r.command))
                .take(num.unwrap_or(5)).collect();
            if top.is_empty() { println!("{}", "No suggestions yet.".yellow()); return; }
            let wc = top.iter().map(|r| r.command.len()).max().unwrap_or(7);
            println!("{}", format!("  {:<wc$}  {:<14}  SCORE", "COMMAND", "ALIAS").cyan().bold());
            println!("{}", format!("  {:-<wc$}  {:-<14}  -----", "", "").dimmed());
            for rec in &top {
                let alias = suggester.suggest(&rec.command).into_iter().next().map(|s| s.alias).unwrap_or_else(|| "—".into());
                println!("  {:<wc$}  {:<14}  {}", rec.command.bold(), alias.cyan(), rec.score);
            }
        }

        Some(Cmd::Search { query, limit }) => {
            let store = load_store(&cfg);
            let mut engine = SearchEngine::new(&cfg);
            for rec in store.all_sorted() { engine.index(rec); }
            let results = engine.search(&query, limit.unwrap_or(20));
            if results.is_empty() { println!("{}", "No results.".yellow()); return; }
            println!("{}", format!("'{}' — {} result(s):", query, results.len()).cyan());
            for (i, r) in results.iter().enumerate() {
                println!("  {:>2}. {} {}", i+1, r.command.bold(), format!("({})", r.score as i64).dimmed());
            }
        }

        Some(Cmd::Stats) => {
            let store = load_store(&cfg);
            let aliased: Vec<String> = aliases.all().into_iter().map(|(_,c)| c).collect();
            let s = miner::compute_stats(&store.all_sorted(), &aliased);
            println!("{}", "  Flux Stats".cyan().bold());
            println!("  Total commands    {}", s.total_commands.to_string().yellow());
            println!("  Unique commands   {}", s.unique_commands.to_string().yellow());
            println!("  Total keystrokes  {}", s.total_keystrokes.to_string().yellow());
            println!("  Keystroke savings {}", s.potential_savings.to_string().green());
            if !s.top_candidates.is_empty() {
                println!("\n{}", "  Top candidates:".cyan());
                for (cmd, freq, saving) in &s.top_candidates {
                    println!("  ×{:<3} {}  ({} saved)", freq, cmd.bold(), saving.to_string().green());
                }
            }
        }

        Some(Cmd::Query { sql }) => {
            let store = load_store(&cfg);
            match query::parse(&sql) {
                Err(e) => println!("{}", format!("Error: {}", e).red()),
                Ok(q) => {
                    let rows = query::execute(&q, &store.all_sorted());
                    if rows.is_empty() { println!("{}", "No results.".yellow()); return; }
                    println!("{}", format!("{} row(s):", rows.len()).cyan());
                    for r in &rows { println!("  {} (freq={}, score={})", r.command.bold(), r.frequency, r.score); }
                }
            }
        }

        Some(Cmd::Suppress { command }) => {
            let mut store = load_store(&cfg);
            store.delete(&command);
            store.save(&cfg.store_path());
            println!("{}", format!("Suppressed: {}", command).yellow());
        }

        Some(Cmd::Predict { command }) => {
            let mut events = vec![];
            if let Some(wal) = Wal::open(&cfg.wal_path(), cfg.max_wal_events) {
                wal.replay(|ev| events.push(ev));
            }
            let sessions = miner::sessionize(&events, 1800);
            let mut dag = miner::WorkflowDag::new();
            dag.ingest(&sessions);
            let preds = dag.predict(&command, 5);
            if preds.is_empty() {
                println!("{}", "No predictions found.".yellow());
            } else {
                println!("{}", format!("Predictions after '{}':", command).cyan());
                for (cmd, prob) in preds {
                    println!("  {:<20} ({:.0}%)", cmd.bold(), prob * 100.0);
                }
            }
        }

        Some(Cmd::Context { cwd }) => {
            let store = load_store(&cfg);
            let target_cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_default().to_string_lossy().to_string());
            let mut results: Vec<_> = store.all_sorted().into_iter()
                .filter(|r| !r.cwd.is_empty() && r.cwd == target_cwd)
                .collect();
            results.sort_by(|a, b| {
                let s_a = a.score + flux::store::context_boost(a, &target_cwd, "");
                let s_b = b.score + flux::store::context_boost(b, &target_cwd, "");
                s_b.cmp(&s_a)
            });
            results.truncate(10);
            if results.is_empty() {
                println!("{}", "No context suggestions found.".yellow());
            } else {
                println!("{}", format!("Context for '{}':", target_cwd).cyan());
                for r in results {
                    println!("  {:<20} (freq={}, score={})", r.command.bold(), r.frequency, r.score);
                }
            }
        }

        Some(Cmd::Tui) => tui::run(&cfg),
    }
}

fn ingest(cfg: &Config, command: &str) {
    let mut store = Store::load(&cfg.store_path());
    let ev = ShellEvent::new(command);
    store.ingest(&ev);
    if let Some(mut wal) = Wal::open(&cfg.wal_path(), cfg.max_wal_events) { wal.append(&ev); }
    store.save(&cfg.store_path());
}

fn load_store(cfg: &Config) -> Store {
    let mut store = Store::load(&cfg.store_path());
    if store.index.is_empty() {
        if let Some(wal) = Wal::open(&cfg.wal_path(), cfg.max_wal_events) { wal.replay(|ev| store.ingest(&ev)); }
    }
    store
}
