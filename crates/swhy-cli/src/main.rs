//! `swhy`: history of Swift and Objective-C code.
//!
//! E0 provides `swhy --version` and `swhy mcp`. The other commands of
//! docs/TZ.md ("Интерфейсы") are added by their stages.

use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Exit code for errors (docs/TZ.md, "Интерфейсы": «3 — ошибка»).
const EXIT_ERROR: u8 = 3;

#[derive(Debug, Parser)]
#[command(
    name = "swhy",
    version,
    about = "Refactoring-aware history of Swift and Objective-C code",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the MCP server on stdin/stdout
    Mcp,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Mcp => match swhy_mcp::run_stdio() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("swhy mcp: {error}");
                ExitCode::from(EXIT_ERROR)
            }
        },
    }
}
