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
    /// E1: the problem measurement (items 1-4) on a dataset repository
    Measure {
        /// Name of the repository in eval/repos.toml
        #[arg(long)]
        repo: String,
        /// Seed of the random samples
        #[arg(long, default_value_t = swhy_eval::E1_SEED)]
        seed: u64,
        /// Print one item to stdout instead of writing the result file
        #[arg(long, value_enum)]
        item: Option<Item>,
        /// Result file: eval/results/E1-measure.json
        #[arg(long, required_unless_present = "item")]
        out: Option<PathBuf>,
        /// The command to record in the result as the way to reproduce it
        #[arg(long, required_unless_present = "item")]
        command: Option<String>,
    },
    /// E1 oracle: sample methods and describe their `git log -L` history
    OraclePool {
        #[arg(long)]
        repo: String,
        /// Methods to sample
        #[arg(long, default_value_t = 1500)]
        size: usize,
        #[arg(long, default_value_t = swhy_eval::E1_SEED + 1)]
        seed: u64,
        /// Output file (JSON list in the order of the draw)
        #[arg(long)]
        out: PathBuf,
    },
    /// E1 oracle: print the review sheet of one method (candidate commits
    /// with the diff of the method)
    OracleSheet {
        #[arg(long)]
        repo: String,
        #[arg(long)]
        path: String,
        #[arg(long)]
        key: String,
        /// Commits where the trace is cut (the method is introduced there)
        #[arg(long)]
        cut: Vec<String>,
        /// Lines of a whole method text to print (diffs are printed in full)
        #[arg(long, default_value_t = 1000)]
        max_lines: usize,
    },
    /// E1 oracle: build eval/oracle/methods/*.json from the labels file
    OracleBuild {
        #[arg(long)]
        repo: String,
        /// eval/oracle/labels/<repo>.tsv
        #[arg(long)]
        labels: PathBuf,
        /// Annotator recorded in the files
        #[arg(long)]
        annotator: String,
        /// eval/oracle/methods or eval/oracle/second
        #[arg(long)]
        out_dir: PathBuf,
    },
    /// E1 oracle: validate the oracle, compute Cohen's kappa, write the result
    OracleCheck {
        #[arg(long, default_value = "eval/oracle/methods")]
        methods: PathBuf,
        #[arg(long, default_value = "eval/oracle/second")]
        second: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        command: String,
    },
}

/// The four measurements of E1, numbered as in docs/TZ.md.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum Item {
    RepoStats,
    BlameNoise,
    HistoryBreaks,
    BugfixShare,
}

fn fail(message: impl std::fmt::Display) -> ExitCode {
    eprintln!("swhy-eval: {message}");
    ExitCode::from(3)
}

fn print_json(value: &impl serde::Serialize) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(error) => fail(error),
    }
}

fn measure(
    repo: &str,
    seed: u64,
    item: Option<Item>,
    out: Option<PathBuf>,
    command: Option<String>,
) -> Result<ExitCode, String> {
    use swhy_eval::measure as m;

    let repo = swhy_eval::repos::Repo::find(repo)?;
    let git = repo.open()?;
    let config = m::MeasureConfig {
        seed,
        ..m::MeasureConfig::default()
    };
    let rev = repo.sha.as_str();
    let error = |error: swhy_eval::gitx::GitError| error.to_string();
    let Some(item) = item else {
        let (params, metrics) = m::measure(&git, repo.measured(), &config).map_err(error)?;
        let (out, command) = (
            out.expect("clap requires --out"),
            command.expect("--command"),
        );
        let result = swhy_eval::result_for_workspace(&command, params, metrics);
        swhy_eval::write_result(&out, &result).map_err(|e| format!("{}: {e}", out.display()))?;
        eprintln!(
            "swhy-eval: measured {} at {rev} -> {}",
            repo.name,
            out.display()
        );
        return Ok(ExitCode::SUCCESS);
    };
    Ok(match item {
        Item::RepoStats => {
            let history = m::commits(&git, rev).map_err(error)?;
            print_json(
                &m::repo_stats(&git, rev, &history, config.huge_commit_files).map_err(error)?,
            )
        }
        Item::BlameNoise => {
            let history = m::commits(&git, rev).map_err(error)?;
            let sources = m::swift_sources(&git, rev).map_err(error)?;
            print_json(
                &m::blame_noise(
                    &git,
                    rev,
                    &sources,
                    &history,
                    config.blame_lines,
                    config.mass_commit_files,
                    seed,
                )
                .map_err(error)?,
            )
        }
        Item::HistoryBreaks => {
            let refs = m::swift_method_refs(&m::swift_sources(&git, rev).map_err(error)?);
            let ends = m::history_ends(&git, rev, &refs, config.break_methods, seed);
            print_json(&m::history_breaks(refs.len(), &ends))
        }
        Item::BugfixShare => {
            let history = m::commits(&git, rev).map_err(error)?;
            print_json(&m::bugfix_share(&history, &m::BUGFIX_KEYWORDS))
        }
    })
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
        Command::Measure {
            repo,
            seed,
            item,
            out,
            command,
        } => measure(&repo, seed, item, out, command).unwrap_or_else(fail),
        other => oracle(other).unwrap_or_else(fail),
    }
}

fn oracle(command: Command) -> Result<ExitCode, String> {
    use swhy_eval::{oracle as o, repos::Repo, trace};

    match command {
        Command::OraclePool {
            repo,
            size,
            seed,
            out,
        } => {
            let repo = Repo::find(&repo)?;
            let entries = o::pool(&repo.open()?, &repo.sha, size, seed)?;
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&out, o::to_json(&entries)).map_err(|e| e.to_string())?;
            eprintln!(
                "swhy-eval: {} pool methods -> {}",
                entries.len(),
                out.display()
            );
        }
        Command::OracleSheet {
            repo,
            path,
            key,
            cut,
            max_lines,
        } => {
            let repo = Repo::find(&repo)?;
            let git = repo.open()?;
            let block = o::LabelBlock {
                id: "sheet".to_string(),
                split: "train".to_string(),
                path,
                key,
                labels: Vec::new(),
                cuts: cut,
            };
            print!(
                "{}",
                trace::sheet(
                    "sheet",
                    &o::trace_block(&git, &repo.sha, &block)?,
                    max_lines
                )
            );
        }
        Command::OracleBuild {
            repo,
            labels,
            annotator,
            out_dir,
        } => {
            let repo = Repo::find(&repo)?;
            let git = repo.open()?;
            let text = std::fs::read_to_string(&labels)
                .map_err(|e| format!("{}: {e}", labels.display()))?;
            let blocks = o::parse_labels(&text)?;
            let built = swhy_eval::gitx::par_map(&blocks, |block| {
                let trace = o::trace_block(&git, &repo.sha, block)?;
                o::build_method(&repo.name, &repo.sha, &annotator, block, &trace)
            });
            std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
            let mut failed = 0;
            for (block, method) in blocks.iter().zip(built) {
                match method {
                    Ok(method) => {
                        let path = out_dir.join(format!("{}.json", block.id));
                        std::fs::write(&path, o::to_json(&method)).map_err(|e| e.to_string())?;
                    }
                    Err(error) => {
                        failed += 1;
                        eprintln!("swhy-eval: {error}");
                    }
                }
            }
            eprintln!(
                "swhy-eval: {} methods, {failed} failed -> {}",
                blocks.len(),
                out_dir.display()
            );
            if failed > 0 {
                return Ok(ExitCode::from(1));
            }
        }
        Command::OracleCheck {
            methods,
            second,
            out,
            command,
        } => {
            let first = o::load_dir(&methods)?;
            let others = if second.is_dir() {
                o::load_dir(&second)?
            } else {
                Vec::new()
            };
            let metrics = o::check(&first, &others)?;
            let ok = metrics.violations.is_empty();
            for violation in &metrics.violations {
                eprintln!("swhy-eval: {violation}");
            }
            let params = o::CheckParams {
                methods_dir: methods.display().to_string(),
                second_dir: second.display().to_string(),
                min_changing_commits: o::MIN_CHANGING_COMMITS,
            };
            let result = swhy_eval::result_for_workspace(&command, params, metrics);
            swhy_eval::write_result(&out, &result)
                .map_err(|e| format!("{}: {e}", out.display()))?;
            eprintln!("swhy-eval: oracle check -> {}", out.display());
            if !ok {
                return Ok(ExitCode::from(1));
            }
        }
        Command::FixtureDeterminism { .. } | Command::Measure { .. } => {
            unreachable!("handled in main")
        }
    }
    Ok(ExitCode::SUCCESS)
}
