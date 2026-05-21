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
        self.commands = events;
    }

    /// Fuzzy search using Jaro-Winkler similarity and BM25-like frequency weighting.
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let mut results = Vec::new();

        for cmd in &self.commands {
            // Calculate semantic similarity (Jaro-Winkler)
            let similarity = jaro_winkler(query, &cmd.command);
            
            // Only consider reasonable matches
            if similarity > 0.6 {
                // Boost score based on how recently it was used (simple recency bias)
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
                let age_seconds = now.saturating_sub(cmd.timestamp);
                let recency_boost = if age_seconds < 86400 { 1.2 } else { 1.0 };

                results.push(SearchResult {
                    event: cmd.clone(),
                    score: similarity * recency_boost,
                });
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().take(limit).collect()
    }
}
