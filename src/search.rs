use std::collections::{HashMap, HashSet};
use crate::config::Config;
use crate::store::Record;

// ── Trie ──────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Node { children: HashMap<u8, Box<Edge>>, ids: Vec<u32> }
struct Edge { label: Vec<u8>, node: Node }

#[derive(Default)]
pub struct Trie { root: Node }

impl Trie {
    pub fn insert(&mut self, text: &str, id: u32) { ins(&mut self.root, text.as_bytes(), id); }
    pub fn prefix(&self, q: &str) -> Vec<u32> { let mut out = vec![]; collect(&self.root, q.as_bytes(), &mut out); out }
}

fn cp(a: &[u8], b: &[u8]) -> usize { a.iter().zip(b).take_while(|(x,y)| x==y).count() }

fn ins(node: &mut Node, key: &[u8], id: u32) {
    if key.is_empty() { node.ids.push(id); return; }
    let first = key[0];
    if let Some(e) = node.children.get_mut(&first) {
        let c = cp(key, &e.label);
        if c == e.label.len() { ins(&mut e.node, &key[c..], id); }
        else {
            let old_label = e.label[c..].to_vec();
            let new_label = e.label[..c].to_vec();
            let old_node  = std::mem::replace(&mut e.node, Node::default());
            let mut split = Node::default();
            split.children.insert(old_label[0], Box::new(Edge { label: old_label, node: old_node }));
            let rem = key[c..].to_vec();
            if rem.is_empty() { split.ids.push(id); }
            else { let mut leaf = Node::default(); leaf.ids.push(id); split.children.insert(rem[0], Box::new(Edge { label: rem, node: leaf })); }
            e.label = new_label; e.node = split;
        }
    } else {
        let mut leaf = Node::default(); leaf.ids.push(id);
        node.children.insert(first, Box::new(Edge { label: key.to_vec(), node: leaf }));
    }
}

fn collect(node: &Node, key: &[u8], out: &mut Vec<u32>) {
    if key.is_empty() { collect_all(node, out); return; }
    if let Some(e) = node.children.get(&key[0]) {
        let c = cp(key, &e.label);
        if c == key.len() { collect_all(&e.node, out); }
        else if c == e.label.len() { collect(&e.node, &key[c..], out); }
    }
}

fn collect_all(node: &Node, out: &mut Vec<u32>) {
    out.extend_from_slice(&node.ids);
    for e in node.children.values() { collect_all(&e.node, out); }
}

// ── BM25 ──────────────────────────────────────────────────────────────────────

struct Bm25 {
    inv: HashMap<String, HashMap<u32, f64>>,
    dl:  HashMap<u32, f64>,
    avg_dl: f64, n: u32, k1: f64, b: f64,
}

impl Bm25 {
    fn new(k1: f64, b: f64) -> Self { Bm25 { inv: HashMap::new(), dl: HashMap::new(), avg_dl: 0.0, n: 0, k1, b } }

    fn add(&mut self, id: u32, text: &str) {
        let toks = tokens(text);
        self.dl.insert(id, toks.len() as f64);
        self.n += 1;
        self.avg_dl = self.dl.values().sum::<f64>() / self.n as f64;
        for t in toks { *self.inv.entry(t).or_default().entry(id).or_insert(0.0) += 1.0; }
    }

    fn score(&self, q: &str) -> Vec<(u32, f64)> {
        let mut scores: HashMap<u32, f64> = HashMap::new();
        for term in tokens(q) {
            let Some(postings) = self.inv.get(&term) else { continue };
            let df = postings.len() as f64;
            let idf = ((self.n as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();
            for (&id, &tf) in postings {
                let dl = *self.dl.get(&id).unwrap_or(&1.0);
                let ntf = tf * (self.k1 + 1.0) / (tf + self.k1 * (1.0 - self.b + self.b * dl / self.avg_dl));
                *scores.entry(id).or_insert(0.0) += idf * ntf;
            }
        }
        let mut v: Vec<_> = scores.into_iter().collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        v
    }
}

fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .filter(|t| !t.is_empty()).map(|t| t.to_lowercase()).collect()
}

// ── Fuzzy ─────────────────────────────────────────────────────────────────────

fn fuzzy_match(q: &str, candidate: &str, max_dist: usize) -> bool {
    let (q, c) = (q.to_lowercase(), candidate.to_lowercase());
    if c.contains(&q) { return true; }
    levenshtein(&q, &c) <= max_dist
}

fn levenshtein(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let (m, n) = (a.len(), b.len());
    if m == 0 { return n; } if n == 0 { return m; }
    if m.abs_diff(n) > 10 { return 11; }
    let mut dp = vec![vec![0usize; n+1]; m+1];
    for i in 0..=m { dp[i][0] = i; } for j in 0..=n { dp[0][j] = j; }
    for i in 1..=m { for j in 1..=n {
        let c = if a[i-1]==b[j-1] { 0 } else { 1 };
        dp[i][j] = (dp[i-1][j]+1).min(dp[i][j-1]+1).min(dp[i-1][j-1]+c);
    }}
    dp[m][n]
}

// ── SearchEngine ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub command: String,
    pub score: f64,
    pub frequency: i32,
}

pub struct SearchEngine {
    trie: Trie,
    bm25: Bm25,
    docs: Vec<Record>,
    text_to_id: HashMap<String, usize>,
}

impl SearchEngine {
    pub fn new(cfg: &Config) -> Self {
        SearchEngine { trie: Trie::default(), bm25: Bm25::new(cfg.bm25_k1, cfg.bm25_b), docs: vec![], text_to_id: HashMap::new() }
    }

    pub fn index(&mut self, rec: &Record) {
        if self.text_to_id.contains_key(&rec.command) { return; }
        let id = self.docs.len() as u32;
        self.trie.insert(&rec.command, id);
        self.bm25.add(id, &rec.command);
        self.text_to_id.insert(rec.command.clone(), self.docs.len());
        self.docs.push(rec.clone());
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            let mut out: Vec<SearchResult> = self.docs.iter().map(|r| SearchResult { command: r.command.clone(), score: r.score as f64, frequency: r.frequency }).collect();
            out.sort_by(|a,b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            out.truncate(limit); return out;
        }
        let prefix_ids: HashSet<u32> = self.trie.prefix(query).into_iter().collect();
        let bm25_map: HashMap<u32, f64> = self.bm25.score(query).into_iter().collect();
        let max_dist = (query.len() / 4).max(1);

        let mut out: Vec<SearchResult> = self.docs.iter().enumerate().filter_map(|(idx, rec)| {
            let id = idx as u32;
            let is_prefix = prefix_ids.contains(&id);
            let bm25 = bm25_map.get(&id).copied().unwrap_or(0.0);
            let is_fuzzy = !is_prefix && bm25 == 0.0 && fuzzy_match(query, &rec.command, max_dist);
            if !is_prefix && bm25 == 0.0 && !is_fuzzy { return None; }
            Some(SearchResult {
                command: rec.command.clone(),
                score: rec.score as f64 + bm25 * 2.0 + if is_prefix { 5.0 } else { 0.0 },
                frequency: rec.frequency,
            })
        }).collect();
        out.sort_by(|a,b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        out.truncate(limit); out
    }
}

// ── AliasSuggester ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Suggestion { pub alias: String, pub command: String, pub reason: String }

pub struct AliasSuggester { existing: HashSet<String>, system: HashSet<String> }

impl AliasSuggester {
    pub fn new(existing_aliases: Vec<String>) -> Self {
        AliasSuggester { existing: existing_aliases.into_iter().collect(), system: system_commands() }
    }

    pub fn suggest(&self, command: &str) -> Vec<Suggestion> {
        let mut out = vec![];
        out.extend(self.semantic(command));
        out.extend(self.abbreviation(command));
        out.extend(self.vowel_removal(command));
        out.extend(self.combined(command));
        out.extend(self.truncated(command));
        let mut seen = HashSet::new();
        out.retain(|s| self.ok(&s.alias) && seen.insert(s.alias.clone()));
        out.sort_by(|a, b| self.priority(b).cmp(&self.priority(a)));
        out
    }

    fn ok(&self, a: &str) -> bool {
        (2..=8).contains(&a.len()) && !self.existing.contains(a) && !self.system.contains(a)
    }

    fn priority(&self, s: &Suggestion) -> i32 {
        let base = if ["Git","Docker","Cargo","NPM","Kubectl"].iter().any(|p| s.reason.starts_with(p)) { 100 }
            else if s.reason == "Abbreviation" { 90 }
            else if s.reason == "Vowel removal" { 80 }
            else if s.reason.contains("combination") { 70 } else { 40 };
        base + 10 - s.alias.len() as i32
    }

    fn semantic(&self, command: &str) -> Vec<Suggestion> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        let (tool, sub) = (parts.first().copied().unwrap_or(""), parts.get(1).copied().unwrap_or(""));
        let hit = match (tool, sub) {
            ("git","status")   => Some(("gs","git status","Git status")),
            ("git","push")     => Some(("gp","git push","Git push")),
            ("git","pull")     => Some(("gl","git pull","Git pull")),
            ("git","log")      => Some(("glg","git log","Git log")),
            ("git","branch")   => Some(("gb","git branch","Git branch")),
            ("git","add")      => Some(("ga","git add","Git add")),
            ("git","commit")   => Some(("gc","git commit","Git commit")),
            ("git","checkout") => Some(("gco","git checkout","Git checkout")),
            ("git","diff")     => Some(("gd","git diff","Git diff")),
            ("git","stash")    => Some(("gst","git stash","Git stash")),
            ("docker","ps")      => Some(("dps","docker ps","Docker ps")),
            ("docker","run")     => Some(("dr","docker run","Docker run")),
            ("docker","build")   => Some(("db","docker build","Docker build")),
            ("docker","exec")    => Some(("de","docker exec","Docker exec")),
            ("docker","compose") => Some(("dc","docker compose","Docker compose")),
            ("cargo","build")  => Some(("cb","cargo build","Cargo build")),
            ("cargo","test")   => Some(("ct","cargo test","Cargo test")),
            ("cargo","run")    => Some(("cr","cargo run","Cargo run")),
            ("cargo","check")  => Some(("cc","cargo check","Cargo check")),
            ("cargo","clippy") => Some(("cl","cargo clippy","Cargo clippy")),
            ("npm"|"pnpm"|"yarn","install") => Some(("ni","npm install","NPM install")),
            ("npm"|"pnpm"|"yarn","run")     => Some(("nr","npm run","NPM run")),
            ("npm"|"pnpm"|"yarn","test")    => Some(("nt","npm test","NPM test")),
            ("kubectl","get")   => Some(("kg","kubectl get","Kubectl get")),
            ("kubectl","apply") => Some(("ka","kubectl apply","Kubectl apply")),
            _ => None,
        };
        if let Some((a,c,r)) = hit { return vec![sg(a,c,r)]; }
        if !sub.is_empty() {
            if let Some(first) = tool.chars().next() {
                return vec![sg(&format!("{}{}", first, sub), command, &format!("{}-{} combination", tool, sub))];
            }
        }
        vec![]
    }

    fn abbreviation(&self, command: &str) -> Vec<Suggestion> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.len() < 2 { return vec![]; }
        let a: String = parts.iter().filter_map(|p| p.chars().next()).collect();
        if (2..=4).contains(&a.len()) { vec![sg(&a, command, "Abbreviation")] } else { vec![] }
    }

    fn vowel_removal(&self, command: &str) -> Vec<Suggestion> {
        let s: String = command.split_whitespace()
            .flat_map(|w| w.chars().filter(|c| !"aeiouAEIOU".contains(*c)).take(3))
            .take(8).collect();
        if s.len() >= 2 && s != command { vec![sg(&s, command, "Vowel removal")] } else { vec![] }
    }

    fn combined(&self, command: &str) -> Vec<Suggestion> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.len() < 2 || parts[1].len() < 2 { return vec![]; }
        let a = format!("{}{}", parts[0].chars().next().unwrap_or('x'), parts[1]);
        vec![sg(&a, command, &format!("{}-{} combination", parts[0], parts[1]))]
    }

    fn truncated(&self, command: &str) -> Vec<Suggestion> {
        let tool = command.split_whitespace().next().unwrap_or("");
        (2..=tool.len().min(5)).filter_map(|len| {
            let t: String = tool.chars().take(len).collect();
            if t != tool { Some(sg(&t, command, &format!("Truncated to {} chars", len))) } else { None }
        }).collect()
    }
}

fn sg(alias: &str, command: &str, reason: &str) -> Suggestion {
    Suggestion { alias: alias.to_string(), command: command.to_string(), reason: reason.to_string() }
}

#[cfg(unix)]
fn system_commands() -> HashSet<String> {
    use std::os::unix::fs::PermissionsExt;
    let mut cmds = HashSet::new();
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for e in entries.flatten() {
                    if let Ok(m) = e.metadata() {
                        if m.is_file() && m.permissions().mode() & 0o111 != 0 {
                            if let Some(n) = e.file_name().to_str() { cmds.insert(n.to_string()); }
                        }
                    }
                }
            }
        }
    }
    cmds
}

#[cfg(windows)]
fn system_commands() -> HashSet<String> {
    let mut cmds = HashSet::new();
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(';') {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for e in entries.flatten() {
                    if let Ok(m) = e.metadata() {
                        if m.is_file() {
                            if let Some(n) = e.file_name().to_str() {
                                let lower = n.to_lowercase();
                                if lower.ends_with(".exe") || lower.ends_with(".cmd") || lower.ends_with(".bat") {
                                    cmds.insert(n.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    cmds
}
