// ─── almanx-storage/src/lib.rs ────────────────────────────────────────────────
//
// Append-only event log stored as newline-delimited JSON (JSONL).
// Each line is one CommandEvent.  Fast appends, full sequential reads.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use almanx_collector::CommandEvent;

pub struct EventLog {
    file_path: PathBuf,
}

impl EventLog {
    pub fn new(path: PathBuf) -> Self {
        Self { file_path: path }
    }

    /// Append a single event to the log.
    /// Creates the file if it doesn't exist.
    pub fn append(&self, event: &CommandEvent) -> anyhow::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)?;
        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string(event)?;
        writeln!(writer, "{}", json)?;
        Ok(())
    }

    /// Read all events from the log.
    /// Silently skips malformed lines.
    pub fn read_all(&self) -> anyhow::Result<Vec<CommandEvent>> {
        if !self.file_path.exists() {
            return Ok(vec![]);
        }
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() { continue; }
            if let Ok(event) = serde_json::from_str(line) {
                events.push(event);
            }
        }
        Ok(events)
    }

    /// Return number of events in the log.
    pub fn count(&self) -> usize {
        self.read_all().map(|v| v.len()).unwrap_or(0)
    }

    /// Compact the log: keep only the last `max_events` events.
    /// Useful to keep the file size bounded.
    pub fn compact(&self, max_events: usize) -> anyhow::Result<()> {
        let mut events = self.read_all()?;
        if events.len() <= max_events {
            return Ok(());
        }
        let start = events.len() - max_events;
        let recent: Vec<CommandEvent> = events.drain(start..).collect();

        // Rewrite file
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.file_path)?;
        let mut writer = BufWriter::new(file);
        for event in &recent {
            let json = serde_json::to_string(event)?;
            writeln!(writer, "{}", json)?;
        }
        Ok(())
    }
}
