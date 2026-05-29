use clap::{ArgEnum, Parser, Subcommand};

#[derive(Parser)]
#[clap(name = "flux", version, about = "Flux — shell intelligence platform")]
pub struct Cli {
    #[clap(subcommand)]
    pub cmd: Option<Cmd>,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// Add a shell alias
    Add { alias: String, #[clap(short='c', long)] command: String },
    /// Remove an alias
    Remove { alias: String },
    /// Change an alias
    Change { old_alias: String, new_alias: String, command: String },
    /// List all aliases
    List,
    /// Suggest aliases for frequent commands
    Suggest { #[clap(short='n', long)] num: Option<usize> },
    /// Fuzzy search command history
    Search { query: String, #[clap(short='l', long)] limit: Option<usize> },
    /// Workflow stats
    Stats,
    /// Query command history
    Query { sql: String },
    /// Suppress a command from suggestions
    Suppress { command: String },
    /// Launch interactive TUI
    Tui,
    /// Predict the next commands you are likely to run
    Predict { command: String },
    /// Contextual command suggestions based on current environment
    Context { cwd: Option<String> },
    #[clap(hide = true)]
    Init { #[clap(arg_enum)] shell: Shell },
}

#[derive(ArgEnum, Clone)]
pub enum Shell { Bash, Zsh, Fish, #[clap(alias="ksh")] Posix }
