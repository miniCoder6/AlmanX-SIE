use std::collections::HashMap;
use crate::store::{Record, ShellEvent};

// ── Sessions ──────────────────────────────────────────────────────────────────

pub struct Session { pub id: String, pub events: Vec<ShellEvent> }

pub fn sessionize(events: &[ShellEvent], gap_secs: i64) -> Vec<Session> {
    let mut sessions: Vec<Session> = vec![];
    let mut cur: Option<Session> = None;
    for ev in events {
        let new = cur.as_ref().map_or(true, |s|
            ev.session_id != s.id || ev.timestamp - s.events.last().map_or(0, |e| e.timestamp) > gap_secs
        );
        if new { if let Some(s) = cur.take() { sessions.push(s); } cur = Some(Session { id: ev.session_id.clone(), events: vec![ev.clone()] }); }
        else if let Some(ref mut s) = cur { s.events.push(ev.clone()); }
    }
    if let Some(s) = cur { sessions.push(s); }
    sessions
}

// ── Workflow DAG ──────────────────────────────────────────────────────────────

pub struct WorkflowDag { adj: HashMap<String, HashMap<String, usize>> }

impl WorkflowDag {
    pub fn new() -> Self { WorkflowDag { adj: HashMap::new() } }

    pub fn ingest(&mut self, sessions: &[Session]) {
        for s in sessions {
            for w in s.events.windows(2) {
                *self.adj.entry(w[0].command.clone()).or_default().entry(w[1].command.clone()).or_insert(0) += 1;
            }
        }
    }

    pub fn predict(&self, current: &str, n: usize) -> Vec<(String, f64)> {
        let Some(ns) = self.adj.get(current) else { return vec![] };
        let total: usize = ns.values().sum();
        let mut v: Vec<(String, f64)> = ns.iter().map(|(cmd, &c)| (cmd.clone(), c as f64 / total as f64)).collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        v.truncate(n); v
    }
}

pub fn mine_workflows(sessions: &[Session], min_freq: usize) -> Vec<(Vec<String>, usize)> {
    let mut ngrams: HashMap<Vec<String>, usize> = HashMap::new();
    
    for s in sessions {
        let cmds: Vec<String> = s.events.iter().map(|e| e.command.clone()).collect();
        for len in 2..=4 {
            for window in cmds.windows(len) {
                *ngrams.entry(window.to_vec()).or_insert(0) += 1;
            }
        }
    }
    
    let mut filtered: Vec<(Vec<String>, usize)> = ngrams.into_iter()
        .filter(|(_, count)| *count >= min_freq).collect();
        
    filtered.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    
    let mut i = 0;
    while i < filtered.len() {
        let (seq, count) = filtered[i].clone();
        if seq.len() > 2 {
            for j in (i + 1)..filtered.len() {
                if is_subslice(&seq, &filtered[j].0) {
                    filtered[j].1 = filtered[j].1.saturating_sub(count);
                }
            }
        }
        i += 1;
    }
    
    filtered.retain(|(_, count)| *count >= min_freq);
    filtered.sort_by(|a, b| b.1.cmp(&a.1));
    filtered
}

fn is_subslice(main: &[String], sub: &[String]) -> bool {
    if sub.len() >= main.len() { return false; }
    main.windows(sub.len()).any(|w| w == sub)
}


// ── Markov Chain ──────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct MarkovChain { transitions: HashMap<String, HashMap<String, u32>> }

impl MarkovChain {
    pub fn observe(&mut self, from: &str, to: &str) {
        *self.transitions.entry(from.to_string()).or_default().entry(to.to_string()).or_insert(0) += 1;
    }

    pub fn predict(&self, current: &str, n: usize) -> Vec<(String, f64)> {
        let Some(counts) = self.transitions.get(current) else { return vec![] };
        let total: u32 = counts.values().sum();
        let mut v: Vec<(String, f64)> = counts.iter().map(|(cmd, &c)| (cmd.clone(), c as f64 / total as f64)).collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        v.truncate(n); v
    }
}

// ── Stats ─────────────────────────────────────────────────────────────────────

pub struct Stats {
    pub total_commands: usize,
    pub unique_commands: usize,
    pub total_keystrokes: usize,
    pub potential_savings: usize,
    pub top_candidates: Vec<(String, i32, usize)>,
}

pub fn compute_stats(records: &[&Record], existing_alias_cmds: &[String]) -> Stats {
    let aliased: std::collections::HashSet<&String> = existing_alias_cmds.iter().collect();
    let total_commands    = records.iter().map(|r| r.frequency as usize).sum();
    let total_keystrokes  = records.iter().map(|r| r.char_len as usize * r.frequency as usize).sum();
    let mut candidates: Vec<(String, i32, usize)> = records.iter()
        .filter(|r| !aliased.contains(&r.command) && r.token_count > 1)
        .map(|r| (r.command.clone(), r.frequency, (r.char_len as usize).saturating_sub(2) * r.frequency as usize))
        .collect();
    candidates.sort_by(|a, b| b.2.cmp(&a.2));
    candidates.truncate(10);
    let potential_savings = candidates.iter().map(|(_, _, s)| s).sum();
    Stats { total_commands, unique_commands: records.len(), total_keystrokes, potential_savings, top_candidates: candidates }
}
