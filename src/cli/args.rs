// ─── cli/args.rs ──────────────────────────────────────────────────────────────
use clap::{Parser, Subcommand as ClapSubcommand, ValueEnum};

/// AlmanX — shell intelligence engine.
///
/// Run without arguments to open the interactive TUI.
#[derive(Debug, Parser)]
#[command(name = "almanx", version, about = "Shell intelligence engine — track, search, and alias your commands", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,
}

#[derive(Debug, ClapSubcommand)]
pub enum Subcommand {
    /// Record a raw shell command (called by shell hooks automatically).
    #[command(hide = true)]
    Record {
        /// The command string(s) to record.
        command: Vec<String>,
        /// Current working directory.
        #[arg(long, default_value = "")]
        cwd: String,
        /// Exit code of the command.
        #[arg(long, default_value_t = 0)]
        exit_code: i32,
        /// Duration in milliseconds.
        #[arg(long, default_value_t = 0)]
        duration: u64,
    },

    /// Fuzzy search your command history.
    Search {
        /// The query string to search for.
        query: String,
        /// Maximum number of results to show.
        #[arg(short, long, default_value_t = 15)]
        limit: usize,
    },

    /// Show productivity analytics and stats.
    Stats,

    /// Show mined workflow patterns (frequently repeated command sequences).
    Workflows {
        /// Minimum frequency to display.
        #[arg(short, long, default_value_t = 2)]
        min_freq: u32,
    },

    /// Suggest alias names for your most-used commands.
    Suggest {
        /// How many suggestions to show.
        #[arg(short, long, default_value_t = 10)]
        num: usize,
    },

    /// Add an alias to your alias file.
    Add {
        /// Short alias name (e.g. `gp`).
        alias: String,
        /// Full command the alias expands to (e.g. `git push`).
        command: Vec<String>,
    },

    /// Remove an alias from your alias file.
    Remove {
        /// The alias name to remove.
        alias: String,
    },

    /// Rename an alias (keeps the same command).
    Rename {
        /// Current alias name.
        old: String,
        /// New alias name.
        new: String,
    },

    /// List all tracked aliases.
    List,

    /// Dismiss a command — stop surfacing it as a suggestion.
    Dismiss {
        /// The exact command text to dismiss.
        command: Vec<String>,
    },

    /// Launch the interactive TUI (default when no subcommand given).
    Tui,

    /// Print the shell integration snippet.
    Init {
        /// Target shell.
        #[arg(value_enum)]
        shell: Shell,
    },
}

/// Supported shells for `almanx init`.
#[derive(Debug, Clone, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}
