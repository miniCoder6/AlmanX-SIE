// ─── main.rs ──────────────────────────────────────────────────────────────────
//
// AlmanX — shell intelligence engine
//
// Architecture:
//   database/   — in-memory command store, scoring, persistence
//   ops/        — alias file I/O, alias suggestion engine
//   cli/        — clap argument definitions
//   tui/        — ratatui interactive TUI
//   shell.rs    — shell init script generator
//   analytics.rs— stats and workflow display

mod analytics;
mod cli;
mod database;
mod ops;
mod shell;
mod tui;

use clap::Parser;
use cli::{Cli, Shell, Subcommand};
use colored::*;
use database::persistence::{
    ensure_data_dir, load_config, load_db, load_deleted, save_db, save_deleted,
};
use ops::alias_file::{add_alias_to_files, get_all_aliases, remove_alias_from_files};
use shell::{init_script, ShellContext};

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", format!("error: {}", e).red());
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    ensure_data_dir()?;

    let cli = Cli::parse();
    let config = load_config();
    let alias_files = config.alias_files.clone();
    let primary_alias_file = alias_files
        .first()
        .cloned()
        .unwrap_or_else(database::persistence::default_alias_path);

    match cli.subcommand {
        // ── No subcommand → launch TUI ────────────────────────────────────────
        None | Some(Subcommand::Tui) => {
            tui::run(primary_alias_file, alias_files)?;
        }

        // ── Record a shell command (called by shell hooks) ────────────────────
        Some(Subcommand::Record { command, cwd, exit_code, duration }) => {
            let text = command.join(" ");
            if text.is_empty() {
                return Ok(());
            }
            let mut db      = load_db();
            let mut deleted = load_deleted();
            db.record(text.clone(), &deleted);
            save_db(&db)?;

            // Also append to the event log for workflow mining and search.
            let storage_path = database::persistence::event_log_path();
            let log = almanx_storage::EventLog::new(storage_path);
            let event = almanx_collector::CommandEvent::new(text, cwd, exit_code, duration);
            let _ = log.append(&event);
        }

        // ── Search command history ────────────────────────────────────────────
        Some(Subcommand::Search { query, limit }) => {
            analytics::search_commands(&query, limit)?;
        }

        // ── Show stats ────────────────────────────────────────────────────────
        Some(Subcommand::Stats) => {
            analytics::show_stats()?;
        }

        // ── Show workflow patterns ────────────────────────────────────────────
        Some(Subcommand::Workflows { min_freq }) => {
            analytics::show_workflows(min_freq)?;
        }

        // ── Suggest aliases ───────────────────────────────────────────────────
        Some(Subcommand::Suggest { num }) => {
            let mut db   = load_db();
            let _deleted = load_deleted();
            let suggester = ops::AliasSuggester::new(&primary_alias_file);
            let commands  = db.top(num);

            if commands.is_empty() {
                println!("{}", "No commands tracked yet. Use your shell for a while first.".yellow());
                return Ok(());
            }

            let w_cmd   = commands.iter().map(|c| c.text.len()).max().unwrap_or(7).max(7);
            let w_alias = 12usize;
            let w_why   = 35usize;

            println!(
                "{} {} {}",
                format!("{:<w$}", "COMMAND",  w = w_cmd).cyan().bold(),
                format!("{:<w$}", "ALIAS",    w = w_alias).cyan().bold(),
                format!("{:<w$}", "REASON",   w = w_why).cyan().bold(),
            );
            println!("{}", "─".repeat(w_cmd + w_alias + w_why + 2).dimmed());

            for cmd in commands {
                let suggestions = suggester.suggest(&cmd.text);
                if let Some(best) = suggestions.first() {
                    println!(
                        "{} {} {}",
                        format!("{:<w$}", cmd.text,      w = w_cmd),
                        format!("{:<w$}", best.alias,    w = w_alias).cyan(),
                        format!("{:<w$}", best.reason,   w = w_why).dimmed(),
                    );
                } else {
                    println!(
                        "{} {} {}",
                        format!("{:<w$}", cmd.text, w = w_cmd),
                        format!("{:<w$}", "—",      w = w_alias).dimmed(),
                        format!("{:<w$}", "no conflict-free suggestion", w = w_why).dimmed(),
                    );
                }
            }
        }

        // ── Add alias ─────────────────────────────────────────────────────────
        Some(Subcommand::Add { alias, command }) => {
            let command_str = command.join(" ");
            add_alias_to_files(&alias_files, &alias, &command_str);
            println!(
                "{} alias {}='{}'",
                "Added:".green().bold(),
                alias.cyan(),
                command_str
            );
        }

        // ── Remove alias ──────────────────────────────────────────────────────
        Some(Subcommand::Remove { alias }) => {
            remove_alias_from_files(&alias_files, &alias);
            println!("{} {}", "Removed alias:".yellow().bold(), alias.cyan());
        }

        // ── Rename alias ──────────────────────────────────────────────────────
        Some(Subcommand::Rename { old, new }) => {
            let all = get_all_aliases(&alias_files);
            if let Some((_, cmd)) = all.into_iter().find(|(a, _)| a == &old) {
                remove_alias_from_files(&alias_files, &old);
                add_alias_to_files(&alias_files, &new, &cmd);
                println!(
                    "{} {} → {}",
                    "Renamed:".green().bold(),
                    old.cyan(),
                    new.cyan()
                );
            } else {
                eprintln!("{} '{}' not found", "error:".red().bold(), old);
            }
        }

        // ── List aliases ──────────────────────────────────────────────────────
        Some(Subcommand::List) => {
            let aliases = get_all_aliases(&alias_files);
            if aliases.is_empty() {
                println!("{}", "No aliases tracked yet.".yellow());
                return Ok(());
            }
            let w = aliases.iter().map(|(a, _)| a.len()).max().unwrap_or(5).max(5);
            println!("{} {}", format!("{:<w$}", "ALIAS", w = w).cyan().bold(), "COMMAND".cyan().bold());
            println!("{}", "─".repeat(w + 4 + 40).dimmed());
            for (alias, cmd) in &aliases {
                println!("{} {}", format!("{:<w$}", alias, w = w).cyan(), cmd);
            }
        }

        // ── Dismiss a command ─────────────────────────────────────────────────
        Some(Subcommand::Dismiss { command }) => {
            let text = command.join(" ");
            let mut db      = load_db();
            let mut deleted = load_deleted();
            db.tombstone(&text, &mut deleted);
            save_db(&db)?;
            save_deleted(&deleted)?;
            println!("{} {}", "Dismissed:".yellow().bold(), text);
        }

        // ── Print shell init snippet ──────────────────────────────────────────
        Some(Subcommand::Init { shell }) => {
            let ctx = ShellContext::load(&primary_alias_file);
            print!("{}", init_script(&shell, &ctx));
        }
    }

    Ok(())
}
