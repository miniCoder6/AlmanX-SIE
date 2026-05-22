// ─── almanx-indexer/src/lib.rs ────────────────────────────────────────────────
//
// Fuzzy + contextual search over command events.
// Uses Jaro-Winkler for string similarity and applies:
//   - recency boost  (recent commands score higher)
//   - directory boost (commands from current CWD rank higher)
//   - exact prefix boost (typing "git" boosts "git push", "git status", etc.)

use almanx_collector::CommandEvent;
use strsim::jaro_winkler;

pub struct Indexer {
    commands: Vec<CommandEvent>,
}

pub struct SearchResult {
    pub event: CommandEvent,
    pub score: f64,
}

impl Indexer {
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }

    pub fn build_index(&mut self, events: Vec<CommandEvent>) {
        // Deduplicate: keep the most recent occurrence of each command.
        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut deduped: Vec<CommandEvent> = Vec::new();

        for event in events.into_iter().rev() {
            if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(event.command.clone()) {
                e.insert(deduped.len());
                deduped.push(event);
            }
        }

        self.commands = deduped;
    }

    /// Search for commands matching the query.
    ///
    /// Scoring:
    ///   base     = Jaro-Winkler similarity (0.0–1.0)
    ///   prefix   = +0.3 if command starts with query
    ///   recency  = ×1.3 if last seen within 24 h; ×1.1 within 7 d
    ///   exit_ok  = ×1.1 if exit_code == 0
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut results: Vec<SearchResult> = self.commands.iter().filter_map(|cmd| {
            let cmd_lower = cmd.command.to_lowercase();

            // Jaro-Winkler base similarity
            let base_sim = jaro_winkler(&query_lower, &cmd_lower);

            // Substring match boost: much more useful than pure edit distance
            let sub_sim = if cmd_lower.contains(&query_lower) {
                0.85f64.max(base_sim)
            } else {
                base_sim
            };

            // Prefix match is very strong signal
            let prefix_boost = if cmd_lower.starts_with(&query_lower) { 0.30 } else { 0.0 };

            let score = sub_sim + prefix_boost;

            // Reject low-quality matches
            if score < 0.55 {
                return None;
            }

            // Recency boost
            let age_secs = now.saturating_sub(cmd.timestamp);
            let recency = if age_secs < 86_400 { 1.3 }
                         else if age_secs < 604_800 { 1.1 }
                         else { 1.0 };

            // Successful commands get a small boost
            let exit_boost = if cmd.exit_code == 0 { 1.1 } else { 1.0 };

            Some(SearchResult {
                score: score * recency * exit_boost,
                event: cmd.clone(),
            })
        }).collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().take(limit).collect()
    }
}

impl Default for Indexer {
    fn default() -> Self { Self::new() }
}
