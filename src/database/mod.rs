// ─── database/mod.rs ──────────────────────────────────────────────────────────
// Public re-exports keep the rest of the codebase import-clean.
pub mod ops;
pub mod persistence;
pub mod scoring;
pub mod structs;

pub use structs::{Command, Database, DeletedCommands};
