// ─── cli/args.rs ──────────────────────────────────────────────────────────────
//
// Clap-derived CLI.  Each subcommand maps 1:1 to a handler in main.rs.

use clap::{Parser, Subcommand as ClapSubcommand, ValueEnum};

/// AlmanX — shell intelligence engine.
///
/// Run without arguments to open the interactive TUI.
#[derive(Debug, Parser)]
#[command(name = "almanx", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,
}

#[derive(Debug, ClapSubcommand)]
pub enum Subcommand {
    /// Record a raw shell command (called by shell hooks, not by users directly).
    #[command(hide = true)]
    Record {
        /// The command string to record.
        command: Vec<String>,
    },

    /// Suggest alias names for your most-used commands.
    Suggest {
        /// How many suggestions to show (default: 10).
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
