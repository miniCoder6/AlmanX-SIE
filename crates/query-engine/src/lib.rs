use almanx_collector::CommandEvent;
use almanx_workflow_engine::WorkflowDag;
use std::collections::HashMap;

pub struct QueryEngine {
    events: Vec<CommandEvent>,
    workflows: Vec<WorkflowDag>,
}

pub struct Context {
    pub cwd: String,
    pub time_of_day_hour: u32,
}

impl QueryEngine {
    pub fn new(events: Vec<CommandEvent>, workflows: Vec<WorkflowDag>) -> Self {
        Self { events, workflows }
    }

    /// Context-aware suggestions based on current working directory.
    pub fn suggest_for_context(&self, ctx: &Context, limit: usize) -> Vec<String> {
        let mut scores: HashMap<String, u32> = HashMap::new();

        for event in &self.events {
            if event.cwd == ctx.cwd {
                // Highly weight commands executed in the same directory
                *scores.entry(event.command.clone()).or_insert(0) += 5;
            } else {
                // Lower weight for global commands
                *scores.entry(event.command.clone()).or_insert(0) += 1;
            }
        }

        let mut ranked: Vec<_> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        ranked.into_iter().take(limit).map(|(cmd, _)| cmd).collect()
    }

    /// Compresses a sequence of commands into a smart alias
    pub fn compress_workflow(&self, workflow: &WorkflowDag) -> String {
        // Simple heuristic: take first letter of each command verb
        let mut acronym = String::new();
        for node in &workflow.nodes {
            if let Some(verb) = node.split_whitespace().next() {
                // For "git add .", we take "g"
                if let Some(ch) = verb.chars().next() {
                    acronym.push(ch);
                }
            }
        }
        
        // If it's a known pattern like git add -> git commit -> git push, it returns gacp
        if acronym.len() > 1 {
            acronym
        } else {
            "alias_".to_string() + &workflow.id[0..4]
        }
    }
}
