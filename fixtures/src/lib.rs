//! Fixture git repositories for tests.
//!
//! A fixture is a script `fixtures/fx-<name>.sh` written with the functions of
//! `fixtures/lib.sh`. [`build`] runs the script and returns the path of the
//! repository in `target/fixtures/<name>`:
//!
//! ```no_run
//! let repo = fixtures::build("fx-rename");
//! assert!(repo.join(".git").is_dir());
//! ```
//!
//! Author, e-mail and dates are fixed by `lib.sh`, so commit SHAs are the same
//! on every machine and in every build.

use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock, PoisonError},
    time::{SystemTime, UNIX_EPOCH},
};

/// Environment variable that names the current test run. Processes with the
/// same value share one build of every fixture. When it is not set, the run is
/// identified by the parent process (the `cargo test` that started the test
/// binaries).
pub const RUN_ID_ENV: &str = "SWHY_FIXTURES_RUN_ID";

/// Git variables that would redirect git away from the repository we name.
const GIT_ENV_REMOVED: [&str; 7] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
];

/// State of [`build`] inside one process.
#[derive(Default)]
struct Built {
    paths: HashMap<String, PathBuf>,
    script_runs: HashMap<String, usize>,
}

fn built() -> std::sync::MutexGuard<'static, Built> {
    static BUILT: OnceLock<Mutex<Built>> = OnceLock::new();
    BUILT
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

/// Directory with `lib.sh` and the `fx-*.sh` scripts.
pub fn scripts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/fixtures`: where [`build`] puts repositories. Follows
/// `CARGO_TARGET_DIR` when it is set.
pub fn root() -> PathBuf {
    let target = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => scripts_dir()
            .parent()
            .expect("fixtures/ lies in the workspace root")
            .join("target"),
    };
    target.join("fixtures")
}

/// Names of all fixtures (`fx-*.sh` without the extension), sorted.
pub fn names() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(scripts_dir())
        .expect("read fixtures directory")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter_map(|file| {
            let name = file.strip_suffix(".sh")?;
            is_valid_name(name).then(|| name.to_string())
        })
        .collect();
    names.sort();
    names
}

fn is_valid_name(name: &str) -> bool {
    name.strip_prefix("fx-").is_some_and(|rest| {
        !rest.is_empty()
            && rest
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    })
}

fn script_path(name: &str) -> PathBuf {
    assert!(
        is_valid_name(name),
        "invalid fixture name '{name}': expected fx-<lowercase letters, digits, dashes>"
    );
    let script = scripts_dir().join(format!("{name}.sh"));
    assert!(
        script.is_file(),
        "unknown fixture '{name}'; available: {}",
        names().join(", ")
    );
    script
}

/// Builds the fixture `name` in `target/fixtures/<name>` and returns that
/// path. The script runs once per test run: later calls, from any thread or
/// test binary of the same run, return the existing repository.
///
/// Tests must not modify the returned repository; use [`build_fresh`] for a
/// private copy.
///
/// # Panics
///
/// When the fixture does not exist or its script fails.
pub fn build(name: &str) -> PathBuf {
    let mut built = built();
    if let Some(path) = built.paths.get(name) {
        return path.clone();
    }
    let (path, ran_script) = build_shared(name);
    if ran_script {
        *built.script_runs.entry(name.to_string()).or_default() += 1;
    }
    built.paths.insert(name.to_string(), path.clone());
    path
}

/// How many times this process ran the script of `name` on behalf of
/// [`build`]: 1, or 0 when another process of the same run built it first.
pub fn script_runs(name: &str) -> usize {
    built().script_runs.get(name).copied().unwrap_or(0)
}

/// Identifier of the build that produced `target/fixtures/<name>`; changes
/// every time the script runs. `None` before the first [`build`].
pub fn build_token(name: &str) -> Option<String> {
    let stamp = fs::read_to_string(stamp_path(name)).ok()?;
    stamp.lines().nth(1).map(str::to_string)
}

fn stamp_path(name: &str) -> PathBuf {
    root().join(format!("{name}.stamp"))
}

fn run_id() -> String {
    match std::env::var(RUN_ID_ENV) {
        Ok(id) if !id.is_empty() => id,
        _ => format!("ppid-{}", std::os::unix::process::parent_id()),
    }
}

/// FNV-1a over `lib.sh` and the fixture script: the repository is rebuilt when
/// either of them changes.
fn scripts_hash(script: &Path) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for path in [scripts_dir().join("lib.sh"), script.to_path_buf()] {
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        for byte in bytes.iter().chain(&[0u8]) {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

/// Returns the repository path and whether the script had to run.
fn build_shared(name: &str) -> (PathBuf, bool) {
    let script = script_path(name);
    let root = root();
    fs::create_dir_all(&root).unwrap_or_else(|e| panic!("create {}: {e}", root.display()));

    // One builder at a time across processes; released when `lock` is dropped.
    let lock_path = root.join(format!("{name}.lock"));
    let lock: File = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .unwrap_or_else(|e| panic!("open {}: {e}", lock_path.display()));
    lock.lock()
        .unwrap_or_else(|e| panic!("lock {}: {e}", lock_path.display()));

    let dest = root.join(name);
    let stamp_path = stamp_path(name);
    let key = format!("run={} scripts={:016x}", run_id(), scripts_hash(&script));
    let current = fs::read_to_string(&stamp_path).ok();
    if dest.join(".git").is_dir() && current.as_deref().and_then(|s| s.lines().next()) == Some(&key)
    {
        return (dest, false);
    }

    let _ = fs::remove_file(&stamp_path);
    if dest.exists() {
        fs::remove_dir_all(&dest).unwrap_or_else(|e| panic!("remove {}: {e}", dest.display()));
    }
    run_script(&script, &dest, &[]);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let stamp = format!("{key}\n{}-{nanos}\n", std::process::id());
    fs::write(&stamp_path, stamp).unwrap_or_else(|e| panic!("write {}: {e}", stamp_path.display()));
    (dest, true)
}

/// Runs the script of `name` into `dest`, always from scratch. `dest` must not
/// exist or must be empty.
///
/// # Panics
///
/// When the fixture does not exist or its script fails.
pub fn build_fresh(name: &str, dest: &Path) {
    build_fresh_with_env(name, dest, &[]);
}

/// [`build_fresh`] with extra environment variables for the script. Exists to
/// check that the machine's environment does not change the result.
pub fn build_fresh_with_env(name: &str, dest: &Path, env: &[(&str, &str)]) {
    run_script(&script_path(name), dest, env);
}

fn run_script(script: &Path, dest: &Path, env: &[(&str, &str)]) {
    let output = Command::new("bash")
        .arg(script)
        .arg(dest)
        .envs(env.iter().copied())
        .output()
        .unwrap_or_else(|e| panic!("run bash {}: {e}", script.display()));
    assert!(
        output.status.success(),
        "fixture script {} failed ({}):\n--- stdout\n{}\n--- stderr\n{}",
        script.display(),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Runs `git -C <repo> <args>` without the machine's git configuration and
/// returns stdout without the trailing newline.
///
/// # Panics
///
/// When git cannot be started or exits with an error.
pub fn git(repo: &Path, args: &[&str]) -> String {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("TZ", "UTC")
        .env("LC_ALL", "C");
    for name in GIT_ENV_REMOVED {
        command.env_remove(name);
    }
    let output = command
        .output()
        .unwrap_or_else(|e| panic!("run git {args:?}: {e}"));
    assert!(
        output.status.success(),
        "git {args:?} in {} failed ({}): {}",
        repo.display(),
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8(output.stdout)
        .expect("git output is UTF-8")
        .trim_end_matches('\n')
        .to_string()
}

/// Full SHA of `HEAD`.
pub fn head_sha(repo: &Path) -> String {
    git(repo, &["rev-parse", "HEAD"])
}

/// SHAs of all commits reachable from any ref, sorted.
pub fn commit_shas(repo: &Path) -> Vec<String> {
    let mut shas: Vec<String> = git(repo, &["rev-list", "--all"])
        .lines()
        .map(str::to_string)
        .collect();
    shas.sort();
    shas
}

/// All refs as `(name, sha)`, sorted by name.
pub fn refs(repo: &Path) -> Vec<(String, String)> {
    let mut refs: Vec<(String, String)> =
        git(repo, &["for-each-ref", "--format=%(refname) %(objectname)"])
            .lines()
            .filter_map(|line| {
                let (name, sha) = line.split_once(' ')?;
                Some((name.to_string(), sha.to_string()))
            })
            .collect();
    refs.sort();
    refs
}
