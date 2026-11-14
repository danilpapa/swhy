//! `swhy-eval`: commands behind the scripts in `eval/`.

use std::{path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "swhy-eval", version, about = "Metric tools of swhy")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// E0: build every fixture several times and compare the commit SHAs
    FixtureDeterminism {
        /// Independent builds of every fixture
        #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u32).range(2..))]
        builds: u32,
        /// Result file, for example eval/results/E0-2026-11-14.json
        #[arg(long)]
        out: PathBuf,
        /// The command to record in the result as the way to reproduce it
        #[arg(long)]
        command: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::FixtureDeterminism {
            builds,
            out,
            command,
        } => {
            let (params, metrics) = swhy_eval::fixture_determinism(builds);
            let ok = metrics.all_identical;
            let result = swhy_eval::result_for_workspace(&command, params, metrics);
            if let Err(error) = swhy_eval::write_result(&out, &result) {
                eprintln!("swhy-eval: write {}: {error}", out.display());
                return ExitCode::from(3);
            }
            eprintln!(
                "swhy-eval: {} of {} fixtures identical over {builds} builds -> {}",
                result.metrics.fixtures_identical,
                result.metrics.fixtures_total,
                out.display()
            );
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
    }
}
