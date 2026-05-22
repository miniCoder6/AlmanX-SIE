// ─── almanx-collector/src/lib.rs ──────────────────────────────────────────────
pub mod shell;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// A single captured shell command event with full telemetry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEvent {
    /// The raw command string.
    pub command: String,
    /// Current working directory when the command ran.
    pub cwd: String,
    /// Unix timestamp (seconds) when the command was executed.
    pub timestamp: i64,
    /// How long the command ran, in milliseconds.
    pub duration_ms: u64,
    /// Exit code (0 = success).
    pub exit_code: i32,
    /// Git branch if the CWD is inside a git repo.
    pub git_branch: Option<String>,
    /// Optional extra environment metadata.
    pub env_metadata: HashMap<String, String>,
}

impl CommandEvent {
    /// Create a new event with the current timestamp and auto-detected git branch.
    pub fn new(command: String, cwd: String, exit_code: i32, duration_ms: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let git_branch = detect_git_branch(&cwd);

        Self {
            command,
            cwd,
            timestamp,
            duration_ms,
            exit_code,
            git_branch,
            env_metadata: HashMap::new(),
        }
    }
}

/// Try to detect the current git branch by reading .git/HEAD.
fn detect_git_branch(cwd: &str) -> Option<String> {
    let mut path = std::path::PathBuf::from(cwd);
    loop {
        let head = path.join(".git").join("HEAD");
        if head.exists() {
            let content = std::fs::read_to_string(&head).ok()?;
            let content = content.trim();
            // "ref: refs/heads/main" → "main"
            if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
                return Some(branch.to_owned());
            }
            // Detached HEAD
            return Some(content.chars().take(8).collect());
        }
        if !path.pop() {
            break;
        }
    }
    None
}
