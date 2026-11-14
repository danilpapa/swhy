//! Benchmark and metric tools of `swhy`.
//!
//! Every metric is computed by a script in `eval/` that runs one command of
//! the `swhy-eval` binary and writes `eval/results/<stage>-<date>.json` with
//! the fields `head_sha`, `command`, `params`, `metrics`.

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use serde::Serialize;

/// Content of one file in `eval/results/`. Holds no generation time: the same
/// input gives the same bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvalResult<P, M> {
    /// `HEAD` of the swhy repository the metric was computed on.
    pub head_sha: String,
    /// Whether the working tree had uncommitted changes at that moment.
    pub worktree_dirty: bool,
    /// The one command that reproduces the file.
    pub command: String,
    pub params: P,
    pub metrics: M,
}

/// Root of the swhy workspace (where `eval/` lives).
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/swhy-eval lies two levels below the workspace root")
        .to_path_buf()
}

/// Wraps `params` and `metrics` with the state of the swhy repository.
pub fn result_for_workspace<P, M>(command: &str, params: P, metrics: M) -> EvalResult<P, M> {
    let root = workspace_root();
    EvalResult {
        head_sha: fixtures::head_sha(&root),
        worktree_dirty: !fixtures::git(&root, &["status", "--porcelain"]).is_empty(),
        command: command.to_string(),
        params,
        metrics,
    }
}

/// Writes `result` as pretty JSON with a trailing newline.
pub fn write_result<P: Serialize, M: Serialize>(
    path: &Path,
    result: &EvalResult<P, M>,
) -> io::Result<()> {
    let mut text = serde_json::to_string_pretty(result).map_err(io::Error::other)?;
    text.push('\n');
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

/// Parameters of [`fixture_determinism`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeterminismParams {
    /// Independent builds of every fixture.
    pub builds: u32,
    pub fixtures: Vec<String>,
}

/// What independent builds of one fixture produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixtureDeterminism {
    /// All builds have the same commits, refs and HEAD.
    pub identical: bool,
    /// HEAD of the first build.
    pub head: String,
    /// Number of commits in the first build.
    pub commits: usize,
    /// Number of distinct HEAD SHAs over all builds (1 when deterministic).
    pub distinct_heads: usize,
}

/// E0 acceptance criterion: «`fixtures::build` дважды даёт одинаковые SHA».
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeterminismMetrics {
    pub fixtures_total: usize,
    pub fixtures_identical: usize,
    pub all_identical: bool,
    pub fixtures: BTreeMap<String, FixtureDeterminism>,
}

/// Builds every fixture `builds` times from scratch and compares the SHAs of
/// all commits and refs.
///
/// # Panics
///
/// When `builds < 2` or a fixture script fails.
pub fn fixture_determinism(builds: u32) -> (DeterminismParams, DeterminismMetrics) {
    assert!(builds >= 2, "determinism needs at least two builds");
    let names = fixtures::names();
    let scratch = fixtures::root()
        .join(".eval")
        .join(std::process::id().to_string());
    let _ = fs::remove_dir_all(&scratch);

    let mut per_fixture = BTreeMap::new();
    for name in &names {
        let mut observed = Vec::new();
        for i in 0..builds {
            let dir = scratch.join(format!("{name}-{i}"));
            fixtures::build_fresh(name, &dir);
            observed.push((
                fixtures::head_sha(&dir),
                fixtures::commit_shas(&dir),
                fixtures::refs(&dir),
            ));
        }
        let (head, commits, _) = &observed[0];
        let mut heads: Vec<&String> = observed.iter().map(|(head, _, _)| head).collect();
        heads.sort();
        heads.dedup();
        per_fixture.insert(
            name.clone(),
            FixtureDeterminism {
                identical: observed.iter().all(|o| o == &observed[0]),
                head: head.clone(),
                commits: commits.len(),
                distinct_heads: heads.len(),
            },
        );
    }
    let _ = fs::remove_dir_all(&scratch);

    let identical = per_fixture.values().filter(|f| f.identical).count();
    let metrics = DeterminismMetrics {
        fixtures_total: per_fixture.len(),
        fixtures_identical: identical,
        all_identical: identical == per_fixture.len(),
        fixtures: per_fixture,
    };
    let params = DeterminismParams {
        builds,
        fixtures: names,
    };
    (params, metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_fixture_is_identical_over_three_builds() {
        let (params, metrics) = fixture_determinism(3);

        assert_eq!(params.builds, 3);
        assert_eq!(params.fixtures, fixtures::names());
        assert!(params.fixtures.contains(&"fx-rename".to_string()));
        assert_eq!(metrics.fixtures_total, params.fixtures.len());
        assert_eq!(metrics.fixtures_identical, metrics.fixtures_total);
        assert!(metrics.all_identical);
        for (name, fixture) in &metrics.fixtures {
            assert!(fixture.identical, "{name}");
            assert_eq!(fixture.distinct_heads, 1, "{name}");
            assert_eq!(fixture.head.len(), 40, "{name}");
            assert!(fixture.commits > 0, "{name}");
        }
        assert_eq!(metrics.fixtures["fx-rename"].commits, 7);
    }

    #[test]
    fn result_file_has_the_four_required_fields_and_no_time() {
        let result = EvalResult {
            head_sha: "0123456789abcdef0123456789abcdef01234567".to_string(),
            worktree_dirty: false,
            command: "eval/e0.sh".to_string(),
            params: DeterminismParams {
                builds: 2,
                fixtures: vec!["fx-rename".to_string()],
            },
            metrics: DeterminismMetrics {
                fixtures_total: 1,
                fixtures_identical: 1,
                all_identical: true,
                fixtures: BTreeMap::from([(
                    "fx-rename".to_string(),
                    FixtureDeterminism {
                        identical: true,
                        head: "917c1cb2dda0bdab223ff8077f935b2b56fb65cf".to_string(),
                        commits: 7,
                        distinct_heads: 1,
                    },
                )]),
            },
        };
        let dir = fixtures::root()
            .join(".eval")
            .join(format!("result-{}", std::process::id()));
        let path = dir.join("nested").join("E0-test.json");
        write_result(&path, &result).unwrap();
        let first = fs::read_to_string(&path).unwrap();
        write_result(&path, &result).unwrap();
        let second = fs::read_to_string(&path).unwrap();
        fs::remove_dir_all(&dir).unwrap();

        assert_eq!(first, second);
        assert_eq!(
            first,
            r#"{
  "head_sha": "0123456789abcdef0123456789abcdef01234567",
  "worktree_dirty": false,
  "command": "eval/e0.sh",
  "params": {
    "builds": 2,
    "fixtures": [
      "fx-rename"
    ]
  },
  "metrics": {
    "fixtures_total": 1,
    "fixtures_identical": 1,
    "all_identical": true,
    "fixtures": {
      "fx-rename": {
        "identical": true,
        "head": "917c1cb2dda0bdab223ff8077f935b2b56fb65cf",
        "commits": 7,
        "distinct_heads": 1
      }
    }
  }
}
"#
        );
    }
}
