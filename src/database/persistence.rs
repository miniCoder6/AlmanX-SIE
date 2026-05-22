// ─── database/persistence.rs ──────────────────────────────────────────────────
use super::structs::{Database, DeletedCommands};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

const DATA_DIR:        &str = ".almanx";
const DB_FILE:         &str = "database.json";
const DELETED_FILE:    &str = "deleted.json";
const CONFIG_FILE:     &str = "config.json";
const DEFAULT_ALIASES: &str = "aliases";
const EVENT_LOG_FILE:  &str = "events.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub alias_files: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            alias_files: vec![default_alias_path()],
        }
    }
}

pub fn data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(DATA_DIR)
}

pub fn db_path()         -> PathBuf { data_dir().join(DB_FILE) }
pub fn deleted_path()    -> PathBuf { data_dir().join(DELETED_FILE) }
pub fn config_path()     -> PathBuf { data_dir().join(CONFIG_FILE) }
pub fn event_log_path()  -> PathBuf { data_dir().join(EVENT_LOG_FILE) }

pub fn default_alias_path() -> String {
    data_dir().join(DEFAULT_ALIASES).to_string_lossy().to_string()
}

pub fn ensure_data_dir() -> std::io::Result<()> {
    let d = data_dir();
    if !d.exists() {
        fs::create_dir_all(&d)?;
    }
    Ok(())
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        return AppConfig::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(cfg: &AppConfig) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(cfg)?;
    fs::write(config_path(), json)?;
    Ok(())
}

pub fn load_db() -> Database {
    let path = db_path();
    if !path.exists() {
        return Database::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_db(db: &Database) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(db)?;
    fs::write(db_path(), json)?;
    Ok(())
}

pub fn load_deleted() -> DeletedCommands {
    let path = deleted_path();
    if !path.exists() {
        return DeletedCommands::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_deleted(d: &DeletedCommands) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(d)?;
    fs::write(deleted_path(), json)?;
    Ok(())
}
