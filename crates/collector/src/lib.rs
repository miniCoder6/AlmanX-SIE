pub mod shell;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEvent {
    pub command: String,
    pub cwd: String,
    pub timestamp: i64,
    pub duration_ms: u64,
    pub exit_code: i32,
    pub git_branch: Option<String>,
    pub env_metadata: HashMap<String, String>,
}
