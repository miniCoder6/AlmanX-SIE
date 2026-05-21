// ─── ops/suggest.rs ───────────────────────────────────────────────────────────
//
// Alias suggestion engine.
//
// Design goals:
//   1. Suggestions are deterministic and explainable — each has a human-
//      readable reason.
//   2. No conflicts with existing aliases or PATH binaries.
//   3. Prioritise semantic shortcuts (e.g. `gs` for `git status`) over
//      mechanical ones (e.g. vowel-stripped versions).
//   4. Return at most one suggestion per command in CLI mode (the best one).

use super::alias_file::get_aliases;
use std::collections::HashSet;

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AliasSuggestion {
    pub alias:   String,
    pub command: String,
    pub reason:  String,
    pub priority: i32,
}

pub struct AliasSuggester {
    existing: HashSet<String>,
    system:   HashSet<String>,
}

// ── Constructor ───────────────────────────────────────────────────────────────

impl AliasSuggester {
    pub fn new(alias_file: &str) -> Self {
        Self {
            existing: get_aliases(alias_file)
                .into_iter()
                .map(|(a, _)| a)
                .collect(),
            system: load_path_commands(),
        }
    }

    /// Return all non-conflicting suggestions, ordered best-first.
    pub fn suggest(&self, command: &str) -> Vec<AliasSuggestion> {
        let mut candidates = Vec::new();

        candidates.extend(semantic_suggestions(command));
        candidates.extend(abbreviation_suggestions(command));
        candidates.extend(vowel_strip_suggestions(command));
        candidates.extend(truncation_suggestions(command));

        // Deduplicate by alias name (keep highest priority).
        candidates.sort_by(|a, b| b.priority.cmp(&a.priority));
        let mut seen = HashSet::new();
        candidates
            .into_iter()
            .filter(|s| !self.conflicts(&s.alias) && seen.insert(s.alias.clone()))
            .collect()
    }

    fn conflicts(&self, alias: &str) -> bool {
        alias.len() < 2
            || self.existing.contains(alias)
            || self.system.contains(alias)
    }
}

// ── Suggestion generators ─────────────────────────────────────────────────────

/// Hardcoded high-quality shortcuts for popular tools.
fn semantic_suggestions(command: &str) -> Vec<AliasSuggestion> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return vec![];
    }
    let (tool, args) = (parts[0], &parts[1..]);

    // Table: (tool, subcommand, extra_args_prefix) → alias
    let table: &[(&str, &str, Option<&str>, &str)] = &[
        // git
        ("git", "status",   None,      "gs"),
        ("git", "add",      Some("."), "gaa"),
        ("git", "add",      None,      "ga"),
        ("git", "commit",   Some("-m"),"gcm"),
        ("git", "commit",   None,      "gc"),
        ("git", "push",     None,      "gp"),
        ("git", "pull",     None,      "gl"),
        ("git", "checkout", Some("-b"),"gcb"),
        ("git", "checkout", None,      "gco"),
        ("git", "branch",   None,      "gb"),
        ("git", "log",      None,      "glg"),
        ("git", "diff",     None,      "gd"),
        ("git", "stash",    None,      "gst"),
        ("git", "rebase",   None,      "grb"),
        // docker
        ("docker", "ps",    None,      "dps"),
        ("docker", "run",   None,      "dr"),
        ("docker", "build", None,      "db"),
        ("docker", "exec",  None,      "de"),
        ("docker", "rm",    None,      "drm"),
        // npm
        ("npm", "install",  None,      "ni"),
        ("npm", "run",      None,      "nr"),
        ("npm", "start",    None,      "ns"),
        ("npm", "test",     None,      "nt"),
        // cargo
        ("cargo", "build",  None,      "cb"),
        ("cargo", "test",   None,      "ct"),
        ("cargo", "run",    None,      "cr"),
        ("cargo", "clippy", None,      "ccl"),
        ("cargo", "fmt",    None,      "cfmt"),
        // kubectl
        ("kubectl", "get",    None,    "kg"),
        ("kubectl", "apply",  None,    "ka"),
        ("kubectl", "delete", None,    "kdel"),
        ("kubectl", "logs",   None,    "kl"),
    ];

    for &(t, sub, extra, alias) in table {
        if tool != t {
            continue;
        }
        let sub_match = args.first().map_or(false, |&s| s == sub);
        if !sub_match {
            continue;
        }
        let extra_match = extra.map_or(true, |e| {
            args.get(1).map_or(false, |&s| s == e)
        });
        if extra_match {
            return vec![AliasSuggestion {
                alias:    alias.to_owned(),
                command:  command.to_owned(),
                reason:   format!("common shorthand for `{} {}`", tool, sub),
                priority: 100,
            }];
        }
    }

    vec![]
}

/// First-letter-of-each-word abbreviation.  e.g. `docker compose up` → `dcu`.
fn abbreviation_suggestions(command: &str) -> Vec<AliasSuggestion> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.len() < 2 {
        return vec![];
    }
    let abbr: String = parts
        .iter()
        .filter_map(|w| w.chars().next())
        .collect();

    if abbr.len() < 2 || abbr.len() > 5 {
        return vec![];
    }
    vec![AliasSuggestion {
        alias:    abbr,
        command:  command.to_owned(),
        reason:   "first-letter abbreviation".to_owned(),
        priority: 70,
    }]
}

/// Remove vowels from each token, join, cap at 6 chars.
/// e.g. `git commit` → `gtcmt`
fn vowel_strip_suggestions(command: &str) -> Vec<AliasSuggestion> {
    let stripped: String = command
        .split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| !"aeiouAEIOU".contains(*c))
                .take(3)
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("");

    let alias = if stripped.len() > 6 {
        stripped[..6].to_owned()
    } else {
        stripped
    };

    if alias.len() < 2 || alias == command {
        return vec![];
    }
    vec![AliasSuggestion {
        alias,
        command: command.to_owned(),
        reason:  "vowel-stripped shorthand".to_owned(),
        priority: 50,
    }]
}

/// Truncate the first word to 2/3/4 chars.
fn truncation_suggestions(command: &str) -> Vec<AliasSuggestion> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    let word = parts[0];
    let mut out = Vec::new();

    for len in 2..=word.len().min(4) {
        let trunc: String = word.chars().take(len).collect();
        if trunc == word {
            break;
        }
        out.push(AliasSuggestion {
            alias:    trunc,
            command:  command.to_owned(),
            reason:   format!("{}-char truncation", len),
            priority: 30 + len as i32,
        });
    }
    out
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn load_path_commands() -> HashSet<String> {
    use std::{env, fs, os::unix::fs::PermissionsExt};

    let mut cmds = HashSet::new();
    if let Ok(path_var) = env::var("PATH") {
        for dir in path_var.split(':') {
            if let Ok(entries) = fs::read_dir(dir) {
                for e in entries.flatten() {
                    if let Ok(meta) = e.metadata() {
                        if meta.is_file() && meta.permissions().mode() & 0o111 != 0 {
                            if let Some(name) = e.file_name().to_str() {
                                cmds.insert(name.to_owned());
                            }
                        }
                    }
                }
            }
        }
    }
    cmds
}
