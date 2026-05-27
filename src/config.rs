use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub alias_file_paths: Vec<String>,
    pub data_dir: String,
    pub socket_path: String,
    pub max_wal_events: usize,
    pub bm25_k1: f64,
    pub bm25_b: f64,
}

impl Default for Config {
    fn default() -> Self {
        let dir = data_dir();
        Config {
            socket_path: dir.join("flux.sock").to_string_lossy().into(),
            alias_file_paths: vec![dir.join("aliases").to_string_lossy().into()],
            data_dir: dir.to_string_lossy().into(),
            max_wal_events: 50_000,
            bm25_k1: 1.5,
            bm25_b: 0.75,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = data_dir().join("config.json");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn ensure_dirs(&self) {
        let _ = std::fs::create_dir_all(&self.data_dir);
    }

    pub fn primary_alias_file(&self) -> &str {
        self.alias_file_paths.first().map(String::as_str).unwrap_or("")
    }

    pub fn store_path(&self) -> String { format!("{}/command_store.json", self.data_dir) }
    pub fn wal_path(&self)   -> String { format!("{}/events.wal",         self.data_dir) }
}

fn data_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".flux")
}
