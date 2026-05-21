use almanx_collector::CommandEvent;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDag {
    /// ID of the workflow (e.g. hash of nodes)
    pub id: String,
    /// Ordered list of sequential commands (the happy path)
    pub nodes: Vec<String>,
    /// How many times this exact sequence was observed
    pub frequency: u32,
}

pub struct WorkflowEngine {
    session_timeout_ms: u64,
}

impl WorkflowEngine {
    pub fn new(session_timeout_ms: u64) -> Self {
        Self { session_timeout_ms }
    }

    /// Mines chronological events to reconstruct frequent workflows
    pub fn mine_workflows(&self, events: &[CommandEvent]) -> Vec<WorkflowDag> {
        if events.is_empty() {
            return vec![];
        }

        // 1. Group events into discrete sessions based on time gaps
        let mut sessions: Vec<Vec<String>> = Vec::new();
        let mut current_session = Vec::new();
        let mut last_time = events[0].timestamp;

        for event in events {
            if (event.timestamp - last_time) as u64 > self.session_timeout_ms {
                if !current_session.is_empty() {
                    sessions.push(current_session);
                    current_session = Vec::new();
                }
            }
            current_session.push(event.command.clone());
            last_time = event.timestamp;
        }
        if !current_session.is_empty() {
            sessions.push(current_session);
        }

        // 2. Extract sequential bigrams/trigrams (n-grams) to find common workflows
        let mut sequence_counts: HashMap<Vec<String>, u32> = HashMap::new();
        
        for session in sessions {
            // Sliding window of size 2 to 5 commands
            for window_size in 2..=5 {
                if session.len() < window_size { continue; }
                for window in session.windows(window_size) {
                    let seq = window.to_vec();
                    *sequence_counts.entry(seq).or_insert(0) += 1;
                }
            }
        }

        // 3. Filter and convert to DAG representations
        let mut workflows = Vec::new();
        for (seq, freq) in sequence_counts {
            // Only consider workflows that happen frequently (e.g., > 2 times)
            if freq > 2 {
                let id = format!("{:x}", md5::compute(seq.join("->")));
                workflows.push(WorkflowDag {
                    id,
                    nodes: seq,
                    frequency: freq,
                });
            }
        }

        // Sort by frequency descending, then by length descending
        workflows.sort_by(|a, b| b.frequency.cmp(&a.frequency).then_with(|| b.nodes.len().cmp(&a.nodes.len())));
        workflows
    }
}
