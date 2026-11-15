//! Read-only access to a git repository through the `git` command line.
//!
//! The E1 measurement describes what stock git answers («до появления
//! инструмента»), so it calls the same commands a developer would. The
//! machine's git configuration is switched off: the numbers must not depend
//! on `blame.ignoreRevsFile`, `diff.renames` and the like.

use std::{
    fmt, fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

/// `xfuncname` for Swift. Git has no built-in driver for `*.swift`, and its
/// default rule (a line that starts with a letter in column 0) never matches
/// an indented method, so `git log -L :<func>:<file>` fails with «no match».
/// A line is a funcname line when the keyword `func` is its first word or
/// follows modifiers and attributes.
pub const SWIFT_XFUNCNAME: &str = "^[ \t]*(.*[^A-Za-z0-9_])?func[ \t]+.*$";

/// A failed git command.
#[derive(Debug)]
pub struct GitError {
    pub args: Vec<String>,
    pub code: Option<i32>,
    pub stderr: String,
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "git {} -> {:?}: {}",
            self.args.join(" "),
            self.code,
            self.stderr.trim()
        )
    }
}

impl std::error::Error for GitError {}

/// One repository and the fixed environment git runs in.
#[derive(Debug, Clone)]
pub struct Git {
    repo: PathBuf,
    attributes: PathBuf,
}

impl Git {
    /// # Panics
    ///
    /// When `repo` is not a git repository or the attributes file cannot be
    /// written.
    pub fn open(repo: &Path) -> Self {
        let dir = fixtures::root()
            .parent()
            .expect("target/fixtures has a parent")
            .join("eval");
        fs::create_dir_all(&dir).expect("create target/eval");
        let attributes = dir.join("swift.gitattributes");
        let wanted = "*.swift diff=swift\n";
        if fs::read_to_string(&attributes).ok().as_deref() != Some(wanted) {
            fs::write(&attributes, wanted).expect("write the attributes file");
        }
        let git = Self {
            repo: repo.to_path_buf(),
            attributes,
        };
        git.text(&["rev-parse", "--git-dir"])
            .unwrap_or_else(|error| panic!("{} is not a git repository: {error}", repo.display()));
        git
    }

    pub fn repo(&self) -> &Path {
        &self.repo
    }

    fn command(&self) -> Command {
        let mut command = Command::new("git");
        for var in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_OBJECT_DIRECTORY",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "GIT_COMMON_DIR",
            "GIT_NAMESPACE",
            "GIT_CONFIG",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_PARAMETERS",
            "GIT_EXTERNAL_DIFF",
            "GIT_DIFF_OPTS",
            "GIT_PAGER",
        ] {
            command.env_remove(var);
        }
        command
            .current_dir(&self.repo)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("TZ", "UTC")
            .arg("--no-pager")
            .arg("-c")
            .arg("core.quotePath=false")
            .arg("-c")
            .arg(format!("core.attributesFile={}", self.attributes.display()))
            .arg("-c")
            .arg(format!("diff.swift.xfuncname={SWIFT_XFUNCNAME}"));
        command
    }

    /// Runs `git <args>` and returns stdout.
    pub fn bytes(&self, args: &[&str]) -> Result<Vec<u8>, GitError> {
        let output = self
            .command()
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("start git");
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(GitError {
                args: args.iter().map(ToString::to_string).collect(),
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        }
    }

    /// Runs `git <args>` and returns stdout as text (invalid UTF-8 replaced).
    pub fn text(&self, args: &[&str]) -> Result<String, GitError> {
        self.bytes(args)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Full SHA of `rev`.
    pub fn resolve(&self, rev: &str) -> Result<String, GitError> {
        self.text(&["rev-parse", "--verify", &format!("{rev}^{{commit}}")])
            .map(|sha| sha.trim().to_string())
    }

    /// `git --version` without the prefix: `2.46.1`.
    pub fn version(&self) -> String {
        let text = self.text(&["--version"]).expect("git --version");
        text.trim()
            .strip_prefix("git version ")
            .unwrap_or(text.trim())
            .to_string()
    }

    /// Contents of the objects `specs` (`<rev>:<path>`), `None` for a missing
    /// one. One `git cat-file --batch` process for the whole list.
    pub fn blobs(&self, specs: &[String]) -> Vec<Option<Vec<u8>>> {
        if specs.is_empty() {
            return Vec::new();
        }
        let mut child = self
            .command()
            .args(["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("start git cat-file");
        let mut stdin = child.stdin.take().expect("stdin is piped");
        let stdout = child.stdout.take().expect("stdout is piped");
        let mut blobs = Vec::with_capacity(specs.len());
        std::thread::scope(|scope| {
            scope.spawn(move || {
                for spec in specs {
                    if writeln!(stdin, "{spec}").is_err() {
                        break;
                    }
                }
            });
            let mut reader = BufReader::new(stdout);
            for _ in specs {
                let mut header = Vec::new();
                reader
                    .read_until(b'\n', &mut header)
                    .expect("read cat-file header");
                let header = String::from_utf8_lossy(&header);
                let size = header
                    .trim_end()
                    .rsplit(' ')
                    .next()
                    .and_then(|size| size.parse::<usize>().ok());
                match size {
                    Some(size) if !header.trim_end().ends_with("missing") => {
                        let mut data = vec![0; size + 1];
                        reader.read_exact(&mut data).expect("read cat-file object");
                        data.truncate(size);
                        blobs.push(Some(data));
                    }
                    _ => blobs.push(None),
                }
            }
        });
        child.wait().expect("wait for git cat-file");
        blobs
    }

    /// One blob as text, `None` when the path does not exist in `rev`.
    pub fn blob_text(&self, rev: &str, path: &str) -> Option<String> {
        self.blobs(&[format!("{rev}:{path}")])
            .pop()
            .flatten()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    }
}

/// Number of worker threads: `SWHY_EVAL_JOBS` or the number of CPUs. Does
/// not affect results, only the wall-clock time.
pub fn jobs() -> usize {
    std::env::var("SWHY_EVAL_JOBS")
        .ok()
        .and_then(|jobs| jobs.parse().ok())
        .filter(|&jobs| jobs > 0)
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, usize::from))
}

/// `f` over `items` on [`jobs`] threads; results keep the order of `items`.
pub fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let mut indexed: Vec<(usize, R)> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..jobs().min(items.len().max(1)))
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(item) = items.get(i) else { break };
                        done.push((i, f(item)));
                    }
                    done
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("worker thread"))
            .collect()
    });
    indexed.sort_by_key(|(i, _)| *i);
    indexed.into_iter().map(|(_, result)| result).collect()
}
