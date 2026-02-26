use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tasuki")]
#[command(about = "タスキ — All of your tasks in your Waybar")]
#[command(version = "0.0.1")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Path to config file (default: ~/.config/tasuki/config.toml)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Increase log verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,
}

#[derive(Subcommand)]
pub enum Command {
    /// Output JSON for Waybar custom module
    Waybar,

    /// Open the interactive TUI (default in terminal)
    Tui,

    /// Quick-add a task from the command line
    Add {
        /// Task text (supports natural language: "Buy milk tomorrow #groceries @obsidian")
        text: Vec<String>,
    },

    /// List tasks to stdout (for scripting)
    List {
        /// Filter: today, upcoming, all, done
        #[arg(default_value = "today")]
        filter: String,

        /// Output format: text, json
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Print the active config (resolved, with defaults)
    Config,

    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}
