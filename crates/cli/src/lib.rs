use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "almanx", version = "1.0", about = "AlmanX Workflow Intelligence Engine")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Launch the interactive TUI
    Tui,
    
    /// Search the command history (Fuzzy + Semantic)
    Search {
        /// The query string to search for
        query: String,
        /// Maximum number of results
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    
    /// Show analytics and productivity stats
    Stats,
    
    /// Replay a mined workflow DAG
    Replay {
        /// The workflow ID or name
        workflow: String,
    },
}

pub fn parse() -> Cli {
    Cli::parse()
}
