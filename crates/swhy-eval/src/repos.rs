//! `eval/repos.toml`: the dataset repositories and their pinned commits.

use std::{fs, path::PathBuf};

use serde::Deserialize;

use crate::{gitx::Git, measure::MeasuredRepo, workspace_root};

/// One `[[repo]]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repo {
    pub name: String,
    pub url: String,
    /// Path of the clone, relative to the workspace root.
    pub path: String,
    pub branch: String,
    /// The pinned HEAD: 40 hexadecimal digits.
    pub sha: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    repo: Vec<Repo>,
}

/// Parses the text of `eval/repos.toml`.
pub fn parse(text: &str) -> Result<Vec<Repo>, String> {
    let file: File = toml::from_str(text).map_err(|error| error.to_string())?;
    for repo in &file.repo {
        let is_sha = repo.sha.len() == 40
            && repo
                .sha
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !is_sha {
            return Err(format!(
                "{}: sha must be 40 lowercase hex digits",
                repo.name
            ));
        }
    }
    Ok(file.repo)
}

/// The repositories of `eval/repos.toml`.
pub fn load() -> Result<Vec<Repo>, String> {
    let path = workspace_root().join("eval/repos.toml");
    let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    parse(&text)
}

impl Repo {
    /// Finds `name` (case-insensitive) in `eval/repos.toml`.
    pub fn find(name: &str) -> Result<Self, String> {
        load()?
            .into_iter()
            .find(|repo| repo.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("no repository '{name}' in eval/repos.toml"))
    }

    pub fn clone_path(&self) -> PathBuf {
        workspace_root().join(&self.path)
    }

    /// Name used in oracle file names: `signal-ios`.
    pub fn slug(&self) -> String {
        self.name.to_ascii_lowercase()
    }

    /// Opens the clone and checks that it is complete and has the pinned
    /// commit.
    pub fn open(&self) -> Result<Git, String> {
        let path = self.clone_path();
        if !path.join(".git").exists() {
            return Err(format!("{}: no clone at {}", self.name, path.display()));
        }
        let git = Git::open(&path);
        let shallow = git.text(&["rev-parse", "--is-shallow-repository"]);
        if shallow.map_err(|e| e.to_string())?.trim() != "false" {
            return Err(format!("{}: the clone is shallow", self.name));
        }
        git.resolve(&self.sha).map_err(|_| {
            format!(
                "{}: pinned commit {} is not in the clone",
                self.name, self.sha
            )
        })?;
        Ok(git)
    }

    pub fn measured(&self) -> MeasuredRepo {
        MeasuredRepo {
            name: self.name.clone(),
            url: self.url.clone(),
            sha: self.sha.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repos_toml_pins_only_signal_ios() {
        // docs/TZ.md, E1: «`eval/repos.toml` закрепляет SHA HEAD только
        // Signal-iOS».
        let repos = load().unwrap();
        assert_eq!(repos.len(), 1);
        let repo = &repos[0];
        assert_eq!(repo.name, "Signal-iOS");
        assert_eq!(repo.url, "https://github.com/signalapp/Signal-iOS");
        assert_eq!(repo.path, "data/repos/Signal-iOS");
        assert_eq!(repo.sha.len(), 40);
        assert_eq!(repo.slug(), "signal-ios");
        assert_eq!(Repo::find("signal-ios").unwrap(), *repo);
        assert!(Repo::find("WordPress-iOS").is_err());
    }

    #[test]
    fn a_short_or_symbolic_sha_is_rejected() {
        let text = |sha: &str| {
            format!(
                "[[repo]]\nname = \"X\"\nurl = \"u\"\npath = \"p\"\nbranch = \"main\"\nsha = \"{sha}\"\n"
            )
        };
        assert!(parse(&text("a690a26d2cfe1bc15f99d73d2ef2054d41ba5e45")).is_ok());
        assert!(parse(&text("a690a26")).is_err());
        assert!(parse(&text("HEAD")).is_err());
        assert!(parse("[[repo]]\nname = \"X\"\n").is_err());
    }
}
