// ─── database/structs.rs ──────────────────────────────────────────────────────
//
// Core data types.  Kept intentionally minimal — only store what the scoring
// formula actually uses.  The BTreeSet gives us sorted iteration for free.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};

// ── Database ──────────────────────────────────────────────────────────────────

/// Central in-memory store.
///
/// `command_list` is a BTreeSet sorted by (score DESC, text ASC) so
/// iterating it always yields the most-valuable commands first — O(n) reads
/// with zero extra sorting cost.
///
/// `index` is a mirror HashMap for O(1) look-ups and mutations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    /// Ordered by score (descending) for fast top-N retrieval.
    pub command_list: BTreeSet<Command>,
    /// Keyed by command text for O(1) access.
    pub index: HashMap<String, Command>,
}

impl Default for Database {
    fn default() -> Self {
        Self {
            command_list: BTreeSet::new(),
            index: HashMap::new(),
        }
    }
}

// ── DeletedCommands ───────────────────────────────────────────────────────────

/// Tombstone set — commands the user explicitly dismissed.
/// We never re-surface tombstoned commands, even if they reappear in history.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DeletedCommands {
    pub set: HashSet<String>,
}

// ── Command ───────────────────────────────────────────────────────────────────

/// A single shell command with usage telemetry.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Command {
    /// Frecency score — see `scoring::compute`.
    pub score: i32,
    /// UNIX timestamp of most recent execution.
    pub last_seen: i64,
    /// How many times this command has been executed.
    pub frequency: u32,
    /// Character-length of the raw command string (used in scoring).
    pub length: u16,
    /// Number of whitespace-separated tokens.
    pub word_count: u8,
    /// The actual shell command text.
    pub text: String,
}

/// Sort by score descending; break ties alphabetically.
impl Ord for Command {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .score
            .cmp(&self.score)
            .then_with(|| self.text.cmp(&other.text))
    }
}
impl PartialOrd for Command {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
