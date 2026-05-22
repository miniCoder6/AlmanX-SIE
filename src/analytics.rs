// ─── analytics.rs ─────────────────────────────────────────────────────────────
//
// Implements:
//   - `almanx search <query>`  — fuzzy search over event log
//   - `almanx stats`           — productivity analytics
//   - `almanx workflows`       — display mined workflow DAGs

use almanx_collector::CommandEvent;
use almanx_indexer::Indexer;
use almanx_storage::EventLog;
use almanx_workflow_engine::WorkflowEngine;
use colored::*;
use std::collections::HashMap;

use crate::database::persistence::event_log_path;

fn load_events() -> anyhow::Result<Vec<CommandEvent>> {
    let log = EventLog::new(event_log_path());
    log.read_all()
}

// ── Search ────────────────────────────────────────────────────────────────────

pub fn search_commands(query: &str, limit: usize) -> anyhow::Result<()> {
    let events = load_events()?;

    if events.is_empty() {
        println!("{}", "No command history yet. Start using your shell!".yellow());
        return Ok(());
    }

    let mut indexer = Indexer::new();
    indexer.build_index(events);
    let results = indexer.search(query, limit);

    if results.is_empty() {
        println!("{} No matches found for '{}'", "→".dimmed(), query.cyan());
        return Ok(());
    }

    println!(
        "  {} {} {}",
        format!("{:<8}", "SCORE").cyan().bold(),
        format!("{:<30}", "COMMAND").cyan().bold(),
        "CONTEXT".cyan().bold()
    );
    println!("{}", "─".repeat(70).dimmed());

    for r in results {
        let cwd_short = shorten_path(&r.event.cwd);
        let branch = r.event.git_branch
            .as_deref()
            .map(|b| format!(" [{}]", b))
            .unwrap_or_default();
        println!(
            "  {} {} {}",
            format!("{:<8.3}", r.score).dimmed(),
            format!("{:<30}", r.event.command).white(),
            format!("{}{}", cwd_short, branch).dimmed()
        );
    }

    Ok(())
}

// ── Stats ─────────────────────────────────────────────────────────────────────

pub fn show_stats() -> anyhow::Result<()> {
    let events = load_events()?;

    if events.is_empty() {
        println!("{}", "No command history yet. Use `almanx init` to set up shell hooks.".yellow());
        return Ok(());
    }

    let total = events.len();

    // Command frequency map
    let mut freq: HashMap<String, u32> = HashMap::new();
    let mut by_dir: HashMap<String, Vec<String>> = HashMap::new();
    let mut exit_failures = 0u32;
    let mut total_duration_ms = 0u64;

    for e in &events {
        *freq.entry(e.command.clone()).or_insert(0) += 1;
        by_dir.entry(e.cwd.clone()).or_default().push(e.command.clone());
        if e.exit_code != 0 {
            exit_failures += 1;
        }
        total_duration_ms += e.duration_ms;
    }

    // Top commands
    let mut top: Vec<(&String, &u32)> = freq.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1));

    // Estimate keystrokes saved (average command length × frequency)
    let potential_savings: u64 = top.iter().take(10).map(|(cmd, &count)| {
        if count > 3 {
            (cmd.len() as u64).saturating_sub(4) * (count as u64 - 1)
        } else {
            0
        }
    }).sum();

    // Most active directories
    let mut dir_counts: Vec<(&String, usize)> = by_dir.iter()
        .map(|(d, cmds)| (d, cmds.len()))
        .collect();
    dir_counts.sort_by(|a, b| b.1.cmp(&a.1));

    let success_rate = if total > 0 {
        100.0 - (exit_failures as f64 / total as f64 * 100.0)
    } else {
        100.0
    };

    // Header
    println!("\n{}", " ╔═══════════════════════════════════╗".cyan());
    println!("{}", " ║      AlmanX Analytics Report      ║".cyan());
    println!("{}\n", " ╚═══════════════════════════════════╝".cyan());

    // Summary
    println!("{}", "── Summary ─────────────────────────────────────────────".cyan().bold());
    stat_line("Total commands tracked", &total.to_string());
    stat_line("Unique commands", &freq.len().to_string());
    stat_line("Success rate", &format!("{:.1}%", success_rate));
    stat_line("Total tracked time", &format_duration(total_duration_ms));
    stat_line("Potential keystrokes saved", &format!("~{} chars with top aliases", potential_savings));
    println!();

    // Top commands
    println!("{}", "── Top 10 Most Used Commands ───────────────────────────".cyan().bold());
    println!(
        "  {} {}",
        format!("{:<5}", "COUNT").dimmed(),
        "COMMAND".dimmed()
    );
    for (cmd, count) in top.iter().take(10) {
        let bar = "█".repeat((*count as usize).min(20));
        println!(
            "  {} {} {}",
            format!("{:<5}", count).yellow(),
            format!("{:<35}", cmd).white(),
            bar.green()
        );
    }
    println!();

    // Most active directories
    println!("{}", "── Most Active Directories ─────────────────────────────".cyan().bold());
    for (dir, count) in dir_counts.iter().take(5) {
        println!(
            "  {} {}",
            format!("{:<5}", count).yellow(),
            shorten_path(dir).white()
        );
    }
    println!();

    // Alias opportunity hint
    if potential_savings > 100 {
        println!("{}", "── Alias Opportunity ───────────────────────────────────".cyan().bold());
        println!("  {} Run {} to see personalized alias suggestions.",
            "💡".yellow(),
            "almanx suggest".cyan().bold()
        );
        println!();
    }

    Ok(())
}

// ── Workflows ─────────────────────────────────────────────────────────────────

pub fn show_workflows(min_freq: u32) -> anyhow::Result<()> {
    let events = load_events()?;

    if events.is_empty() {
        println!("{}", "No command history yet.".yellow());
        return Ok(());
    }

    // 30-minute session timeout
    let engine = WorkflowEngine::new(30 * 60 * 1000);
    let workflows = engine.mine_workflows(&events);

    let shown: Vec<_> = workflows.iter()
        .filter(|w| w.frequency >= min_freq)
        .take(20)
        .collect();

    if shown.is_empty() {
        println!("{}", format!(
            "No workflows found with frequency ≥ {}. Use your shell more and try again.",
            min_freq
        ).yellow());
        return Ok(());
    }

    println!("\n{}", " ╔══════════════════════════════════════╗".cyan());
    println!("{}", " ║     Mined Workflow Patterns (DAG)    ║".cyan());
    println!("{}\n", " ╚══════════════════════════════════════╝".cyan());

    println!("{}", "── Frequent Command Sequences ──────────────────────────".cyan().bold());
    println!(
        "  {} {}",
        format!("{:<6}", "FREQ").dimmed(),
        "WORKFLOW SEQUENCE".dimmed()
    );
    println!("{}", "─".repeat(65).dimmed());

    for wf in shown {
        let chain = wf.nodes.join(" → ");
        let alias_hint = make_workflow_alias(&wf.nodes);
        println!(
            "  {} {}",
            format!("{:<6}", wf.frequency).yellow().bold(),
            chain.white()
        );
        println!(
            "        {} Suggested alias: {}",
            "↳".dimmed(),
            alias_hint.cyan()
        );
        println!();
    }

    println!("{}", "── Tip ─────────────────────────────────────────────────".cyan().bold());
    println!("  Use {} to create a workflow alias:", "almanx add".cyan().bold());
    println!("  {}", "almanx add gac 'git add . && git commit -m'".dimmed());

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn stat_line(label: &str, value: &str) {
    println!("  {:<35} {}", label.dimmed(), value.white().bold());
}

fn shorten_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy();
        if path.starts_with(home_str.as_ref()) {
            return path.replacen(home_str.as_ref(), "~", 1);
        }
    }
    if path.len() > 35 {
        format!("...{}", &path[path.len() - 32..])
    } else {
        path.to_owned()
    }
}

fn format_duration(ms: u64) -> String {
    let secs = ms / 1000;
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

fn make_workflow_alias(nodes: &[String]) -> String {
    // Build acronym from first letter of the first word of each command
    let acronym: String = nodes.iter()
        .filter_map(|n| n.split_whitespace().next())
        .filter_map(|w| w.chars().next())
        .collect();
    if acronym.len() >= 2 && acronym.len() <= 6 {
        acronym
    } else {
        "workflow".to_string()
    }
}
