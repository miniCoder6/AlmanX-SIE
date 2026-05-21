// ─── ops/alias_file.rs ────────────────────────────────────────────────────────
//
// Read/write shell alias files.
//
// Alias file format (same as what shells understand):
//   alias gp='git push'
//   alias gs='git status'

use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

// ── Read ──────────────────────────────────────────────────────────────────────

/// Parse all `alias name='command'` lines from a file.
/// Returns (alias_name, command_text) pairs in declaration order.
pub fn get_aliases(path: &str) -> Vec<(String, String)> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    content
        .lines()
        .filter_map(parse_alias_line)
        .collect()
}

fn parse_alias_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    let rest = line.strip_prefix("alias ")?;
    let eq = rest.find('=')?;
    let name = rest[..eq].trim().to_owned();
    let raw_value = rest[eq + 1..].trim();

    // Strip surrounding single or double quotes.
    let value = if (raw_value.starts_with('\'') && raw_value.ends_with('\''))
        || (raw_value.starts_with('"') && raw_value.ends_with('"'))
    {
        raw_value[1..raw_value.len() - 1].to_owned()
    } else {
        raw_value.to_owned()
    };

    if name.is_empty() || value.is_empty() {
        return None;
    }
    Some((name, value))
}

// ── Write helpers ─────────────────────────────────────────────────────────────

/// Append `alias name='command'` to the file.
/// Creates the file if it doesn't exist.
pub fn add_alias(path: &str, name: &str, command: &str) -> anyhow::Result<()> {
    // Create parent dirs if needed.
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "alias {}='{}'", name, command.replace('\'', "'\\''"))?;
    Ok(())
}

/// Remove every line defining `alias name='...'` from the file.
pub fn remove_alias(path: &str, name: &str) -> anyhow::Result<()> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(()), // nothing to remove
    };

    let prefix = format!("alias {}=", name);
    let filtered: String = content
        .lines()
        .filter(|l| !l.trim().starts_with(&prefix))
        .map(|l| format!("{}\n", l))
        .collect();

    fs::write(path, filtered)?;
    Ok(())
}

/// Convenience: apply `add_alias` across multiple alias files.
pub fn add_alias_to_files(files: &[String], name: &str, command: &str) {
    for f in files {
        if let Err(e) = add_alias(f, name, command) {
            eprintln!("warn: could not write to {}: {}", f, e);
        }
    }
}

/// Convenience: apply `remove_alias` across multiple alias files.
pub fn remove_alias_from_files(files: &[String], name: &str) {
    for f in files {
        if let Err(e) = remove_alias(f, name) {
            eprintln!("warn: could not update {}: {}", f, e);
        }
    }
}

/// Merge aliases from all tracked files, deduplicating by name (first wins).
pub fn get_all_aliases(files: &[String]) -> Vec<(String, String)> {
    let mut seen = HashMap::new();
    let mut result = Vec::new();
    for f in files {
        for (name, cmd) in get_aliases(f) {
            if seen.insert(name.clone(), ()).is_none() {
                result.push((name, cmd));
            }
        }
    }
    result
}
