//! `swhy-fixtures build <name>` builds a fixture the same way tests do and
//! prints where it is; `swhy-fixtures list` prints the fixture names.

use std::process::ExitCode;

const USAGE: &str = "usage: swhy-fixtures build <fx-name> | swhy-fixtures list";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["list"] => {
            for name in fixtures::names() {
                println!("{name}");
            }
            ExitCode::SUCCESS
        }
        ["build", name] => {
            if !fixtures::names().iter().any(|known| known == name) {
                eprintln!(
                    "unknown fixture '{name}'; available: {}",
                    fixtures::names().join(", ")
                );
                return ExitCode::from(2);
            }
            let repo = fixtures::build(name);
            println!("path={}", repo.display());
            println!("head={}", fixtures::head_sha(&repo));
            println!("build={}", fixtures::build_token(name).unwrap_or_default());
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
