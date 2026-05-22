// ─── almanx-workflow-engine/src/lib.rs ────────────────────────────────────────
//
// Mines chronological command events to reconstruct frequent workflow patterns.
// A "workflow" is a frequently repeated sequence of commands (n-gram mining).

use almanx_collector::CommandEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A mined workflow pattern: an ordered sequence of commands seen together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDag {
    /// Stable ID (MD5 of the joined nodes).
    pub id: String,
    /// The sequential commands in this workflow.
    pub nodes: Vec<String>,
    /// How many times this exact sequence was observed.
    pub frequency: u32,
}

pub struct WorkflowEngine {
    /// Sessions are split on time gaps larger than this (milliseconds).
    session_timeout_ms: u64,
}

impl WorkflowEngine {
    pub fn new(session_timeout_ms: u64) -> Self {
        Self { session_timeout_ms }
    }

    /// Mine the event stream for repeated command sequences.
    pub fn mine_workflows(&self, events: &[CommandEvent]) -> Vec<WorkflowDag> {
        if events.is_empty() {
            return vec![];
        }

        // 1. Sessionize events by time gap
        let sessions = self.sessionize(events);

        // 2. Count n-grams (window 2–4 commands)
        let mut sequence_counts: HashMap<Vec<String>, u32> = HashMap::new();

        for session in &sessions {
            for window_size in 2..=4usize {
                if session.len() < window_size { continue; }
                for window in session.windows(window_size) {
                    // Skip sessions with duplicate consecutive commands (noise)
                    if window.windows(2).any(|w| w[0] == w[1]) { continue; }
                    *sequence_counts.entry(window.to_vec()).or_insert(0) += 1;
                }
            }
        }

        // 3. Convert to WorkflowDag, filter by frequency
        let mut workflows: Vec<WorkflowDag> = sequence_counts
            .into_iter()
            .filter(|(_, freq)| *freq >= 2)
            .map(|(seq, freq)| {
                let id = format!("{:x}", md5::compute(seq.join("->")));
                WorkflowDag { id, nodes: seq, frequency: freq }
            })
            .collect();

        // 4. Remove sub-sequences: if "A B C" exists, don't also show "A B"
        workflows = remove_subsumed(workflows);

        // 5. Sort by frequency desc, then length desc (longer = more specific)
        workflows.sort_by(|a, b| {
            b.frequency.cmp(&a.frequency)
                .then_with(|| b.nodes.len().cmp(&a.nodes.len()))
        });

        workflows
    }

    fn sessionize(&self, events: &[CommandEvent]) -> Vec<Vec<String>> {
        let mut sessions: Vec<Vec<String>> = Vec::new();
        let mut current: Vec<String> = Vec::new();
        let mut last_time = events[0].timestamp;

        for event in events {
            // Convert timestamp diff to ms (timestamps are in seconds)
            let gap_ms = ((event.timestamp - last_time).abs() as u64).saturating_mul(1000);
            if gap_ms > self.session_timeout_ms && !current.is_empty() {
                sessions.push(std::mem::take(&mut current));
            }
            current.push(event.command.clone());
            last_time = event.timestamp;
        }
        if !current.is_empty() {
            sessions.push(current);
        }
        sessions
    }
}

/// Remove n-grams that are strict sub-sequences of a longer, more-frequent one.
fn remove_subsumed(mut workflows: Vec<WorkflowDag>) -> Vec<WorkflowDag> {
    // Sort longest first
    workflows.sort_by(|a, b| b.nodes.len().cmp(&a.nodes.len()));

    let mut keep: Vec<WorkflowDag> = Vec::new();

    'outer: for wf in workflows {
        for longer in &keep {
            if longer.frequency >= wf.frequency {
                // Check if wf.nodes is a sub-slice of longer.nodes
                if longer.nodes.windows(wf.nodes.len()).any(|w| w == wf.nodes.as_slice()) {
                    continue 'outer;
                }
            }
        }
        keep.push(wf);
    }

    keep
}
