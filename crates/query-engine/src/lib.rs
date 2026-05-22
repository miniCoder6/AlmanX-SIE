// ─── almanx-query-engine/src/lib.rs ──────────────────────────────────────────
//
// Context-aware command suggestions.
// Given the current working directory and time of day, suggests the most
// relevant commands the user is likely to want next.

use almanx_collector::CommandEvent;
use almanx_workflow_engine::WorkflowDag;
use std::collections::HashMap;

pub struct QueryEngine {
    events:    Vec<CommandEvent>,
    workflows: Vec<WorkflowDag>,
}

pub struct Context {
    pub cwd:             String,
    pub time_of_day_hour: u32,
}

impl QueryEngine {
    pub fn new(events: Vec<CommandEvent>, workflows: Vec<WorkflowDag>) -> Self {
        Self { events, workflows }
    }

    /// Return the top `limit` commands most likely to be relevant in `ctx`.
    ///
    /// Scoring:
    ///   +5 if command was run in the same directory
    ///   +1 for every other occurrence (global familiarity)
    pub fn suggest_for_context(&self, ctx: &Context, limit: usize) -> Vec<String> {
        let mut scores: HashMap<String, u32> = HashMap::new();

        for event in &self.events {
            if event.cwd == ctx.cwd {
                *scores.entry(event.command.clone()).or_insert(0) += 5;
            } else {
                *scores.entry(event.command.clone()).or_insert(0) += 1;
            }
        }

        let mut ranked: Vec<(String, u32)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        ranked.into_iter().take(limit).map(|(cmd, _)| cmd).collect()
    }

    /// Given a workflow, suggest a sensible shell alias for it.
    ///
    /// Strategy: take the first letter of the first word of each node.
    /// "git add . → git commit → git push" → "gacp" (g,a=add,c,p)
    pub fn compress_workflow(&self, workflow: &WorkflowDag) -> String {
        let acronym: String = workflow.nodes.iter()
            .filter_map(|node| node.split_whitespace().next())
            .filter_map(|word| word.chars().next())
            .collect();

        if acronym.len() >= 2 && acronym.len() <= 6 {
            acronym
        } else if workflow.id.len() >= 4 {
            format!("wf_{}", &workflow.id[..4])
        } else {
            "wf_alias".to_string()
        }
    }

    /// What would the user most likely run NEXT, given they just ran `last_cmd`?
    /// Uses transition probabilities from the workflow DAGs.
    pub fn predict_next(&self, last_cmd: &str) -> Vec<String> {
        let mut predictions: HashMap<String, u32> = HashMap::new();

        for wf in &self.workflows {
            for (i, node) in wf.nodes.iter().enumerate() {
                if node == last_cmd {
                    if let Some(next) = wf.nodes.get(i + 1) {
                        *predictions.entry(next.clone()).or_insert(0) += wf.frequency;
                    }
                }
            }
        }

        let mut ranked: Vec<(String, u32)> = predictions.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        ranked.into_iter().take(3).map(|(cmd, _)| cmd).collect()
    }
}
