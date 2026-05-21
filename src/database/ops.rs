// ─── database/ops.rs ──────────────────────────────────────────────────────────
//
// All mutations on `Database` live here.  The invariant: `command_list` and
// `index` must always be in sync — every helper that touches one must touch
// both.

use super::{
    scoring::{compute, decay_all},
    structs::{Command, Database, DeletedCommands},
};
use std::time::{SystemTime, UNIX_EPOCH};

/// Soft threshold for total score.  When exceeded we decay everything.
const DECAY_THRESHOLD: i64 = 10_000;

// ── Internal helpers ──────────────────────────────────────────────────────────

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Remove a command from both `command_list` and `index`.
fn remove_from_both(db: &mut Database, key: &str) {
    if let Some(cmd) = db.index.remove(key) {
        db.command_list.remove(&cmd);
    }
}

/// Insert a command into both `command_list` and `index`.
fn insert_to_both(db: &mut Database, cmd: Command) {
    db.command_list.insert(cmd.clone());
    db.index.insert(cmd.text.clone(), cmd);
}

// ── Public API ────────────────────────────────────────────────────────────────

impl Database {
    // ── Record a command execution ──────────────────────────────────────────

    /// Called every time the user runs a shell command.
    ///
    /// Rules:
    /// - Ignore tombstoned commands (user dismissed them).
    /// - Ignore trivially short single-word commands (≤5 chars, 1 word).
    /// - If already tracked, bump frequency and timestamp.
    /// - Otherwise create a fresh entry.
    pub fn record(&mut self, text: String, deleted: &DeletedCommands) {
        if deleted.set.contains(&text) {
            return;
        }

        if let Some(existing) = self.index.get(&text).cloned() {
            // Update existing entry.
            remove_from_both(self, &text);
            let mut updated = existing;
            updated.frequency += 1;
            updated.last_seen = now_unix();
            updated.score = compute(&updated);
            insert_to_both(self, updated);
        } else {
            // New entry — apply minimum-length filter.
            let words = text.split_whitespace().count() as u8;
            let length = text.split_whitespace().map(|w| w.len()).sum::<usize>() as u16;
            if length <= 5 && words == 1 {
                return;
            }

            let mut cmd = Command {
                score: 0,
                last_seen: now_unix(),
                frequency: 1,
                length,
                word_count: words,
                text: text.clone(),
            };
            cmd.score = compute(&cmd);
            insert_to_both(self, cmd);
        }

        self.maybe_decay();
    }

    // ── Tombstone a command ─────────────────────────────────────────────────

    /// Remove a command from suggestions and add it to the tombstone set so
    /// it is never surfaced again.
    pub fn tombstone(&mut self, text: &str, deleted: &mut DeletedCommands) {
        if deleted.set.insert(text.to_owned()) {
            remove_from_both(self, text);
        }
    }

    // ── Retrieve top-N commands ─────────────────────────────────────────────

    /// Return the top `n` commands by score (highest first).
    /// Recomputes scores before returning so results are always fresh.
    pub fn top(&mut self, n: usize) -> Vec<&Command> {
        self.refresh_scores();
        self.command_list.iter().take(n).collect()
    }

    // ── Internal housekeeping ───────────────────────────────────────────────

    /// Recompute every score in place (needed when time has passed).
    fn refresh_scores(&mut self) {
        let keys: Vec<String> = self.index.keys().cloned().collect();
        for key in keys {
            if let Some(cmd) = self.index.get(&key).cloned() {
                let new_score = compute(&cmd);
                if new_score != cmd.score {
                    self.command_list.remove(&cmd);
                    let mut updated = cmd;
                    updated.score = new_score;
                    self.command_list.insert(updated.clone());
                    self.index.insert(key, updated);
                }
            }
        }
    }

    /// Total score across all commands (used for decay threshold check).
    fn total_score(&self) -> i64 {
        self.index.values().map(|c| c.score as i64).sum()
    }

    /// If total score has grown too large, decay all frequencies by 50%.
    fn maybe_decay(&mut self) {
        if self.total_score() > DECAY_THRESHOLD {
            let commands = std::mem::take(&mut self.command_list)
                .into_iter()
                .collect::<Vec<_>>();
            self.index.clear();

            for cmd in decay_all(commands.into_iter()) {
                insert_to_both(self, cmd);
            }
        }
    }
}
