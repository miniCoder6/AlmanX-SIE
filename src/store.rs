use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

// ── helpers ──────────────────────────────────────────────────────────────────

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

// ── scoring ───────────────────────────────────────────────────────────────────

pub fn frecency(last_access: i64, frequency: i32, char_len: i32) -> i32 {
    let mult = match now() - last_access {
        a if a <= 3_600   => 4.0f64,
        a if a <= 86_400  => 2.0,
        a if a <= 604_800 => 0.5,
        _                 => 0.25,
    };
    (mult * (char_len as f64).powf(0.6) * frequency as f64) as i32
}

pub fn context_boost(rec: &Record, cwd: &str, branch: &str) -> i32 {
    let mut n = 0;
    if !rec.cwd.is_empty() && rec.cwd == cwd { n += 30; }
    if !rec.git_branch.is_empty() && rec.git_branch == branch { n += 20; }
    let current_hour = ((now() % 86400) / 3600) as usize;
    if rec.hours[current_hour] > 0 {
        let total_for_cmd: u32 = rec.hours.iter().map(|&x| x as u32).sum();
        if total_for_cmd > 0 {
            let ratio = rec.hours[current_hour] as f64 / total_for_cmd as f64;
            n += (ratio * 25.0) as i32;
        }
    }
    n
}

// ── ShellEvent ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellEvent {
    pub command: String,
    pub timestamp: i64,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub session_id: String,
}

impl ShellEvent {
    pub fn new(command: &str) -> Self {
        ShellEvent {
            command: command.to_string(),
            timestamp: now(),
            cwd: None, git_branch: None,
            session_id: "cli".into(),
        }
    }
}

// ── CommandRecord ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Record {
    pub command: String,
    pub frequency: i32,
    pub last_access: i64,
    pub char_len: i32,
    pub token_count: i32,
    pub score: i32,
    pub cwd: String,
    pub git_branch: String,
    #[serde(default = "default_hours")]
    pub hours: [u16; 24],
}

fn default_hours() -> [u16; 24] {
    [0; 24]
}

impl Record {
    fn new(text: &str, ts: i64, cwd: &str, branch: &str) -> Self {
        let char_len = text.split_whitespace().map(|t| t.len()).sum::<usize>() as i32;
        let token_count = text.split_whitespace().count() as i32;
        let mut hours = default_hours();
        hours[((ts % 86400) / 3600) as usize] += 1;
        Record {
            command: text.to_string(), frequency: 1, last_access: ts,
            char_len, token_count, score: frecency(ts, 1, char_len),
            cwd: cwd.to_string(), git_branch: branch.to_string(),
            hours,
        }
    }

    fn touch(&mut self, ts: i64, cwd: &str, branch: &str) {
        self.frequency += 1; self.last_access = ts;
        self.cwd = cwd.to_string(); self.git_branch = branch.to_string();
        self.score = frecency(ts, self.frequency, self.char_len);
        let hour = ((ts % 86400) / 3600) as usize;
        self.hours[hour] = self.hours[hour].saturating_add(1);
    }

    pub fn refresh_score(&mut self) {
        self.score = frecency(self.last_access, self.frequency, self.char_len);
    }
}

impl Ord for Record {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.score.cmp(&self.score).then(self.command.cmp(&other.command))
    }
}
impl PartialOrd for Record {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}

// ── CommandStore ──────────────────────────────────────────────────────────────

pub struct Store {
    pub records: BTreeSet<Record>,
    pub index: HashMap<String, Record>,
    pub deleted: HashSet<String>,
    total_score: i64,
}

impl Default for Store {
    fn default() -> Self {
        Store { records: BTreeSet::new(), index: HashMap::new(), deleted: HashSet::new(), total_score: 0 }
    }
}

impl Store {
    pub fn ingest(&mut self, ev: &ShellEvent) {
        let text = ev.command.trim();
        if text.is_empty() || self.deleted.contains(text) { return; }
        let char_len: usize = text.split_whitespace().map(|t| t.len()).sum();
        if char_len <= 5 && text.split_whitespace().count() == 1 { return; }

        let cwd = ev.cwd.as_deref().unwrap_or("");
        let branch = ev.git_branch.as_deref().unwrap_or("");

        if let Some(rec) = self.index.remove(text) {
            self.records.remove(&rec);
            self.total_score -= rec.score as i64;
            let mut updated = rec;
            updated.touch(ev.timestamp, cwd, branch);
            self.total_score += updated.score as i64;
            self.records.insert(updated.clone());
            self.index.insert(text.to_string(), updated);
        } else {
            let rec = Record::new(text, ev.timestamp, cwd, branch);
            self.total_score += rec.score as i64;
            self.records.insert(rec.clone());
            self.index.insert(text.to_string(), rec);
        }

        if self.total_score > 50_000 { self.decay(); }
    }

    pub fn delete(&mut self, text: &str) {
        self.deleted.insert(text.to_string());
        if let Some(rec) = self.index.remove(text) {
            self.records.remove(&rec);
            self.total_score -= rec.score as i64;
        }
    }

    pub fn all_sorted(&self) -> Vec<&Record> { self.records.iter().collect() }

    fn decay(&mut self) {
        let mut to_remove = vec![];
        for (k, rec) in self.index.iter_mut() {
            rec.frequency /= 2;
            if rec.frequency < 1 { to_remove.push(k.clone()); } else { rec.refresh_score(); }
        }
        for k in to_remove { self.index.remove(&k); }
        self.records.clear(); self.total_score = 0;
        for rec in self.index.values() {
            self.total_score += rec.score as i64;
            self.records.insert(rec.clone());
        }
    }

    pub fn load(path: &str) -> Self {
        std::fs::read_to_string(path).ok()
            .and_then(|s| serde_json::from_str::<Snapshot>(&s).ok())
            .map(|snap| {
                let mut store = Store::default();
                store.deleted = snap.deleted;
                for rec in snap.records {
                    store.total_score += rec.score as i64;
                    store.index.insert(rec.command.clone(), rec.clone());
                    store.records.insert(rec);
                }
                store
            })
            .unwrap_or_default()
    }

    pub fn save(&self, path: &str) {
        let snap = Snapshot { records: self.index.values().cloned().collect(), deleted: self.deleted.clone() };
        if let Some(p) = std::path::Path::new(path).parent() { let _ = std::fs::create_dir_all(p); }
        if let Ok(json) = serde_json::to_string_pretty(&snap) { let _ = std::fs::write(path, json); }
    }
}

#[derive(Serialize, Deserialize)]
struct Snapshot { records: Vec<Record>, deleted: HashSet<String> }

// ── WAL ───────────────────────────────────────────────────────────────────────

pub struct Wal {
    path: String,
    writer: File,
    pub count: usize,
    max: usize,
}

impl Wal {
    pub fn open(path: &str, max: usize) -> Option<Self> {
        if let Some(p) = std::path::Path::new(path).parent() { let _ = std::fs::create_dir_all(p); }
        let count = if std::path::Path::new(path).exists() {
            BufReader::new(File::open(path).ok()?).lines().count()
        } else { 0 };
        let writer = OpenOptions::new().create(true).append(true).open(path).ok()?;
        Some(Wal { path: path.to_string(), writer, count, max })
    }

    pub fn append(&mut self, ev: &ShellEvent) {
        if let Ok(line) = serde_json::to_string(ev) {
            let _ = writeln!(self.writer, "{}", line);
            let _ = self.writer.flush();
            self.count += 1;
        }
    }

    pub fn replay(&self, mut f: impl FnMut(ShellEvent)) {
        let Ok(file) = File::open(&self.path) else { return };
        for line in BufReader::new(file).lines().flatten() {
            if let Ok(ev) = serde_json::from_str::<ShellEvent>(&line) { f(ev); }
        }
    }

    pub fn needs_compaction(&self) -> bool { self.count > self.max }

    pub fn compact(&mut self) {
        let Ok(file) = File::open(&self.path) else { return };
        let mut lines: Vec<String> = BufReader::new(file).lines().flatten().collect();
        if lines.len() > self.max { lines.drain(0..lines.len() - self.max); }
        let tmp = format!("{}.tmp", self.path);
        if let Ok(mut f) = File::create(&tmp) {
            for line in &lines { let _ = writeln!(f, "{}", line); }
            let _ = std::fs::rename(&tmp, &self.path);
            self.count = lines.len();
            if let Ok(w) = OpenOptions::new().append(true).open(&self.path) { self.writer = w; }
        }
    }
}

// ── AliasStore ────────────────────────────────────────────────────────────────

pub struct Aliases {
    pub paths: Vec<String>,
}

impl Aliases {
    pub fn new(paths: Vec<String>) -> Self { Aliases { paths } }

    pub fn all(&self) -> Vec<(String, String)> {
        self.paths.iter().flat_map(|p| read_aliases(p)).collect()
    }

    pub fn add(&self, alias: &str, command: &str) {
        if let Some(p) = self.paths.first() {
            let mut list = read_aliases(p);
            if !list.iter().any(|(a, _)| a == alias) {
                list.push((alias.to_string(), command.to_string()));
                write_aliases(p, &list);
            }
        }
    }

    pub fn remove(&self, alias: &str) {
        for p in &self.paths {
            let mut list = read_aliases(p);
            let before = list.len();
            list.retain(|(a, _)| a != alias);
            if list.len() != before { write_aliases(p, &list); break; }
        }
    }

    pub fn change(&self, old: &str, new: &str, command: &str) {
        self.remove(old);
        self.add(new, command);
    }
}

fn read_aliases(path: &str) -> Vec<(String, String)> {
    let Ok(file) = File::open(path) else { let _ = File::create(path); return vec![]; };
    BufReader::new(file).lines().flatten().filter_map(|line| {
        let rest = line.trim().strip_prefix("alias ")?;
        let eq = rest.find('=')?;
        let alias = rest[..eq].trim().to_string();
        let mut cmd = rest[eq+1..].trim();
        if (cmd.starts_with('\'') && cmd.ends_with('\'')) || (cmd.starts_with('"') && cmd.ends_with('"')) {
            cmd = &cmd[1..cmd.len()-1];
        }
        Some((alias, cmd.to_string()))
    }).collect()
}

fn write_aliases(path: &str, aliases: &[(String, String)]) {
    if let Some(p) = std::path::Path::new(path).parent() { let _ = std::fs::create_dir_all(p); }
    if let Ok(mut f) = File::create(path) {
        for (alias, cmd) in aliases { let _ = writeln!(f, "alias {}='{}'", alias, cmd); }
    }
}
