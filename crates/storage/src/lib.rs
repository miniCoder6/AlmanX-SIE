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

    pub fn append(&self, event: &CommandEvent) -> anyhow::Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)?;
        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string(event)?;
        writeln!(writer, "{}", json)?;
        Ok(())
    }

    pub fn read_all(&self) -> anyhow::Result<Vec<CommandEvent>> {
        let file = File::open(&self.file_path);
        if let Err(e) = file {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(vec![]);
            }
            return Err(e.into());
        }
        let file = file.unwrap();
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if let Ok(event) = serde_json::from_str(&line) {
                events.push(event);
            }
        }
        Ok(events)
    }
}
