// ─── ops/mod.rs ───────────────────────────────────────────────────────────────
pub mod alias_file;
pub mod suggest;

pub use alias_file::{add_alias, get_aliases, remove_alias};
pub use suggest::{AliasSuggestion, AliasSuggester};
