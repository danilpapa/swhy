//! E1, «Замер проблемы (N6, до появления инструмента)»: what stock git
//! answers on a real repository, before `swhy` exists.
//!
//! Four measurements, numbered as in docs/TZ.md: repository statistics,
//! blame noise, history breaks of `git log -L :<func>:<file>`, and the rough
//! share of bugfix commits by keywords.

use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};

use serde::Serialize;

use crate::{
    gitx::{Git, GitError, par_map},
    rng,
    scan::{self, SourceLang},
};

/// Keywords of a bugfix message: the weak-signal list of docs/TZ.md, E8.
pub const BUGFIX_KEYWORDS: [&str; 17] = [
    "fix",
    "bug",
    "crash",
    "regression",
    "hotfix",
    "EXC_BAD_ACCESS",
    "leak",
    "retain cycle",
    "force unwrap",
    "main thread",
    "deadlock",
    "race",
    "nil",
    "краш",
    "баг",
    "исправ",
    "падени",
];

/// Share with four decimals; 0 when the denominator is 0.
pub fn share(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    (part as f64 / whole as f64 * 10_000.0).round() / 10_000.0
}

/// One commit of the history with the files it changed. A merge commit has
/// no changes of its own and gets an empty list, as in `git log`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRecord {
    pub sha: String,
    pub parents: Vec<String>,
    /// Committer date, `YYYY-MM-DD`.
    pub date: String,
    /// Subject and body.
    pub message: String,
    pub files: u32,
}

impl CommitRecord {
    pub fn subject(&self) -> &str {
        self.message.lines().next().unwrap_or("")
    }

    pub fn is_merge(&self) -> bool {
        self.parents.len() > 1
    }
}

/// Every commit reachable from `rev`, newest first.
pub fn commits(git: &Git, rev: &str) -> Result<Vec<CommitRecord>, GitError> {
    let log = git.text(&[
        "log",
        "--no-renames",
        "--name-only",
        "--format=%x01%H%x09%P%x09%cs%x00%B%x00",
        rev,
    ])?;
    Ok(log
        .split('\u{1}')
        .skip(1)
        .filter_map(|record| {
            let mut parts = record.splitn(3, '\0');
            let header: Vec<&str> = parts.next()?.split('\t').collect();
            let message = parts.next()?.trim_end().to_string();
            let files = parts
                .next()?
                .lines()
                .filter(|line| !line.is_empty())
                .count();
            Some(CommitRecord {
                sha: (*header.first()?).to_string(),
                parents: header
                    .get(1)?
                    .split_whitespace()
                    .map(str::to_string)
                    .collect(),
                date: (*header.get(2)?).to_string(),
                message,
                files: files as u32,
            })
        })
        .collect())
}

/// A commit that changed more files than the threshold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HugeCommit {
    pub sha: String,
    pub date: String,
    pub files: u32,
    pub subject: String,
}

/// Number of Objective-C files by extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObjcFiles {
    pub total: usize,
    pub h: usize,
    pub m: usize,
    pub mm: usize,
}

/// Item 1: «Статистика репозитория: коммиты, годы, число файлов Swift и
/// ObjC, коммиты с изменением больше 1000 файлов.»
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepoStats {
    pub commits: usize,
    pub merge_commits: usize,
    pub first_commit_date: String,
    pub last_commit_date: String,
    pub first_year: String,
    pub last_year: String,
    pub commits_by_year: BTreeMap<String, usize>,
    pub files: usize,
    pub swift_files: usize,
    pub objc_files: ObjcFiles,
    pub huge_commits: usize,
    /// Newest first.
    pub huge_commit_list: Vec<HugeCommit>,
}

/// Paths of all files in the tree of `rev`.
pub fn tree_paths(git: &Git, rev: &str) -> Result<Vec<String>, GitError> {
    Ok(git
        .text(&["ls-tree", "-r", "--name-only", rev])?
        .lines()
        .map(str::to_string)
        .collect())
}

pub fn repo_stats(
    git: &Git,
    rev: &str,
    history: &[CommitRecord],
    huge_commit_files: u32,
) -> Result<RepoStats, GitError> {
    let paths = tree_paths(git, rev)?;
    let with_extension = |extension: &str| {
        paths
            .iter()
            .filter(|path| path.rsplit_once('.').is_some_and(|(_, e)| e == extension))
            .count()
    };
    let (h, m, mm) = (
        with_extension("h"),
        with_extension("m"),
        with_extension("mm"),
    );
    let mut commits_by_year = BTreeMap::new();
    for commit in history {
        *commits_by_year
            .entry(commit.date[..4].to_string())
            .or_default() += 1;
    }
    let first = history.iter().map(|c| c.date.as_str()).min().unwrap_or("");
    let last = history.iter().map(|c| c.date.as_str()).max().unwrap_or("");
    let huge_commit_list: Vec<HugeCommit> = history
        .iter()
        .filter(|commit| commit.files > huge_commit_files)
        .map(|commit| HugeCommit {
            sha: commit.sha.clone(),
            date: commit.date.clone(),
            files: commit.files,
            subject: commit.subject().to_string(),
        })
        .collect();
    Ok(RepoStats {
        commits: history.len(),
        merge_commits: history.iter().filter(|c| c.is_merge()).count(),
        first_commit_date: first.to_string(),
        last_commit_date: last.to_string(),
        first_year: first.chars().take(4).collect(),
        last_year: last.chars().take(4).collect(),
        commits_by_year,
        files: paths.len(),
        swift_files: with_extension("swift"),
        objc_files: ObjcFiles {
            total: h + m + mm,
            h,
            m,
            mm,
        },
        huge_commits: huge_commit_list.len(),
        huge_commit_list,
    })
}

/// What one mode of `git blame` says about the sampled lines.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BlameMode {
    /// Lines whose commit is technical: whitespace only or a mass commit.
    pub technical: usize,
    pub technical_share: f64,
    /// The commit changed only whitespace in the file (`git diff -w` empty).
    pub whitespace_only: usize,
    /// The commit changed more files than the threshold.
    pub mass_commit: usize,
    /// Distinct commits the sampled lines are attributed to.
    pub distinct_commits: usize,
}

/// Item 2: «Шум blame».
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BlameNoise {
    pub swift_files: usize,
    pub swift_lines: usize,
    pub sampled_lines: usize,
    /// Sampled lines that are empty or whitespace only.
    pub sampled_blank_lines: usize,
    /// `git blame -w`
    pub blame_w: BlameMode,
    /// `git blame -w -M -C -C`
    pub blame_w_m_c_c: BlameMode,
    /// Lines the two modes attribute to different commits.
    pub modes_differ: usize,
    pub modes_differ_share: f64,
}

/// The commit `git blame` attributes one line to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlameOrigin {
    pub sha: String,
    /// Path of the file in that commit.
    pub path: String,
    /// Parent commit and the path there; `None` when the commit added the file.
    pub previous: Option<(String, String)>,
}

/// Runs `git blame <flags> --line-porcelain <rev> -- <path>` on the whole
/// file and returns the origin of every line (index 0 is line 1). The whole
/// file is blamed because `-M` and `-C` judge blocks of lines: a single line
/// under `-L n,n` is almost never long enough to count as moved.
pub fn blame_file(
    git: &Git,
    rev: &str,
    path: &str,
    flags: &[&str],
) -> Result<Vec<BlameOrigin>, GitError> {
    let mut args = vec!["blame"];
    args.extend_from_slice(flags);
    args.extend_from_slice(&["--line-porcelain", rev, "--", path]);
    let output = git.bytes(&args)?;
    let mut origins = Vec::new();
    let mut current: Option<BlameOrigin> = None;
    for line in output.split(|&b| b == b'\n') {
        if line.first() == Some(&b'\t') {
            origins.extend(current.take());
            continue;
        }
        let line = String::from_utf8_lossy(line);
        match &mut current {
            None => {
                let sha = line.split(' ').next().unwrap_or("");
                if sha.len() >= 40 && sha.bytes().all(|b| b.is_ascii_hexdigit()) {
                    current = Some(BlameOrigin {
                        sha: sha.to_string(),
                        path: String::new(),
                        previous: None,
                    });
                }
            }
            Some(origin) => {
                if let Some(path) = line.strip_prefix("filename ") {
                    origin.path = path.to_string();
                } else if let Some(previous) = line.strip_prefix("previous ") {
                    origin.previous = previous
                        .split_once(' ')
                        .map(|(sha, path)| (sha.to_string(), path.to_string()));
                }
            }
        }
    }
    Ok(origins)
}

/// «изменение только пробелов (`git diff -w` пуст)»: the file in the commit
/// differs from its previous version only in whitespace. A commit that adds
/// the file is not whitespace only.
pub fn is_whitespace_only(git: &Git, origin: &BlameOrigin) -> Result<bool, GitError> {
    let Some((previous_sha, previous_path)) = &origin.previous else {
        return Ok(false);
    };
    let before = format!("{previous_sha}:{previous_path}");
    let after = format!("{}:{}", origin.sha, origin.path);
    Ok(git.bytes(&["diff", "-w", &before, &after])?.is_empty())
}

/// Swift files of `rev` with their text, in path order.
pub fn swift_sources(git: &Git, rev: &str) -> Result<Vec<(String, String)>, GitError> {
    let mut paths: Vec<String> = tree_paths(git, rev)?
        .into_iter()
        .filter(|path| path.ends_with(".swift"))
        .collect();
    paths.sort();
    let specs: Vec<String> = paths.iter().map(|path| format!("{rev}:{path}")).collect();
    Ok(paths
        .into_iter()
        .zip(git.blobs(&specs))
        .filter_map(|(path, blob)| Some((path, String::from_utf8_lossy(&blob?).into_owned())))
        .collect())
}

pub fn blame_noise(
    git: &Git,
    rev: &str,
    sources: &[(String, String)],
    history: &[CommitRecord],
    lines: usize,
    mass_commit_files: u32,
    seed: u64,
) -> Result<BlameNoise, GitError> {
    // All lines of all Swift files, numbered in path order.
    let counts: Vec<usize> = sources
        .iter()
        .map(|(_, text)| text.lines().count())
        .collect();
    let total: usize = counts.iter().sum();
    let mut picked = rng::sample(total, lines, seed);
    picked.sort_unstable();
    let mut by_file: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let (mut file, mut offset) = (0, 0);
    for index in picked {
        while index >= offset + counts[file] {
            offset += counts[file];
            file += 1;
        }
        by_file.entry(file).or_default().push(index - offset);
    }
    let files: Vec<(usize, Vec<usize>)> = by_file.into_iter().collect();
    let blank = files
        .iter()
        .flat_map(|(file, lines)| lines.iter().map(move |&line| (*file, line)))
        .filter(|&(file, line)| {
            sources[file]
                .1
                .lines()
                .nth(line)
                .is_some_and(|text| text.trim().is_empty())
        })
        .count();

    let changed_files: HashMap<&str, u32> =
        history.iter().map(|c| (c.sha.as_str(), c.files)).collect();
    let whitespace_cache: Mutex<HashMap<BlameOrigin, bool>> = Mutex::new(HashMap::new());
    let whitespace_only = |origin: &BlameOrigin| -> Result<bool, GitError> {
        if let Some(&known) = whitespace_cache.lock().expect("cache").get(origin) {
            return Ok(known);
        }
        let value = is_whitespace_only(git, origin)?;
        whitespace_cache
            .lock()
            .expect("cache")
            .insert(origin.clone(), value);
        Ok(value)
    };

    // Per sampled line: (origin under -w, origin under -w -M -C -C).
    let blamed = par_map(&files, |(file, lines)| {
        let path = &sources[*file].0;
        let plain = blame_file(git, rev, path, &["-w"])?;
        let copies = blame_file(git, rev, path, &["-w", "-M", "-C", "-C"])?;
        lines
            .iter()
            .map(|&line| match (plain.get(line), copies.get(line)) {
                (Some(a), Some(b)) => Ok((a.clone(), b.clone())),
                _ => Err(GitError {
                    args: vec!["blame".to_string(), path.clone()],
                    code: None,
                    stderr: format!("no blame entry for line {}", line + 1),
                }),
            })
            .collect::<Result<Vec<_>, GitError>>()
    });
    let mut pairs = Vec::new();
    for result in blamed {
        pairs.extend(result?);
    }

    let mode = |origins: Vec<&BlameOrigin>| -> Result<BlameMode, GitError> {
        let (mut technical, mut whitespace, mut mass) = (0, 0, 0);
        let mut distinct: Vec<&str> = origins.iter().map(|o| o.sha.as_str()).collect();
        distinct.sort_unstable();
        distinct.dedup();
        for origin in &origins {
            let is_whitespace = whitespace_only(origin)?;
            let is_mass =
                changed_files.get(origin.sha.as_str()).copied().unwrap_or(0) > mass_commit_files;
            whitespace += usize::from(is_whitespace);
            mass += usize::from(is_mass);
            technical += usize::from(is_whitespace || is_mass);
        }
        Ok(BlameMode {
            technical,
            technical_share: share(technical, origins.len()),
            whitespace_only: whitespace,
            mass_commit: mass,
            distinct_commits: distinct.len(),
        })
    };
    let differ = pairs.iter().filter(|(a, b)| a.sha != b.sha).count();
    Ok(BlameNoise {
        swift_files: sources.len(),
        swift_lines: total,
        sampled_lines: pairs.len(),
        sampled_blank_lines: blank,
        blame_w: mode(pairs.iter().map(|(a, _)| a).collect())?,
        blame_w_m_c_c: mode(pairs.iter().map(|(_, b)| b).collect())?,
        modes_differ: differ,
        modes_differ_share: share(differ, pairs.len()),
    })
}

/// A method of the sampled revision, addressable by `git log -L`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MethodRef {
    pub path: String,
    pub name: String,
    pub line: u32,
}

/// Every `func` of the Swift files, one per (file, name): `git log -L
/// :<func>:<file>` takes the first match in the file, so overloads after the
/// first one cannot be addressed. Operators and escaped names are left out.
pub fn swift_method_refs(sources: &[(String, String)]) -> Vec<MethodRef> {
    let mut refs = Vec::new();
    for (path, text) in sources {
        let mut seen: Vec<String> = Vec::new();
        for method in scan::methods(SourceLang::Swift, text) {
            let plain = method
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_');
            if plain && !seen.contains(&method.name) {
                seen.push(method.name.clone());
                refs.push(MethodRef {
                    path: path.clone(),
                    name: method.name,
                    line: method.decl_line,
                });
            }
        }
    }
    refs
}

/// The `<func>` of `git log -L :<func>:<file>` for a Swift method: a basic
/// regular expression without `:` (the separator of the option).
pub fn funcname_regex(name: &str) -> String {
    format!("func[ \t][ \t]*{name}[ \t]*[(<]")
}

/// One commit of a `git log -L` history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RangeCommit {
    pub sha: String,
    /// Path of the file after the commit.
    pub path: String,
    /// Path before the commit; `None` when the commit added the file.
    pub previous_path: Option<String>,
}

/// `git log -L :<func>:<file>` from `rev`, newest first.
pub fn log_funcname(
    git: &Git,
    rev: &str,
    method: &MethodRef,
) -> Result<Vec<RangeCommit>, GitError> {
    let range = format!(":{}:{}", funcname_regex(&method.name), method.path);
    let output = git.text(&["log", "--format=%x01%H", "-L", &range, rev])?;
    Ok(output
        .split('\u{1}')
        .skip(1)
        .map(|record| {
            let mut lines = record.lines();
            let sha = lines.next().unwrap_or("").to_string();
            let (mut before, mut after) = (None, String::new());
            for line in lines {
                if let Some(path) = line.strip_prefix("--- ") {
                    before = path.strip_prefix("a/").map(str::to_string);
                } else if let Some(path) = line.strip_prefix("+++ ") {
                    after = path.strip_prefix("b/").unwrap_or(path).to_string();
                    break;
                }
            }
            RangeCommit {
                sha,
                path: after,
                previous_path: before,
            }
        })
        .collect())
}

/// Where the history of one sampled method ends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HistoryEnd {
    pub method: MethodRef,
    /// Commits in the `git log -L` history; 0 when git failed.
    pub commits: usize,
    pub last_commit: Option<String>,
    /// The last commit added the file.
    pub ends_at_file_add: bool,
    /// Files deleted by the last commit that hold a method of the same name.
    pub deleted_with_same_name: Vec<String>,
    /// Paths the range went through, oldest last (for the oracle selection).
    pub paths: Vec<String>,
    pub error: Option<String>,
}

impl HistoryEnd {
    /// «Если в том же коммите удалён файл с методом того же имени — это
    /// обрыв на переезде.»
    pub fn is_break(&self) -> bool {
        self.ends_at_file_add && !self.deleted_with_same_name.is_empty()
    }

    pub fn from_objc(&self) -> bool {
        self.deleted_with_same_name
            .iter()
            .any(|path| SourceLang::of_path(path) == Some(SourceLang::ObjC))
    }
}

type DeletedMethods = Arc<Vec<(String, Vec<String>)>>;

/// Finds where `git log -L :<func>:<file>` ends for each method.
pub struct BreakFinder<'a> {
    git: &'a Git,
    rev: &'a str,
    /// Commit → deleted source files with the names of their methods.
    deleted: Mutex<HashMap<String, DeletedMethods>>,
}

impl<'a> BreakFinder<'a> {
    pub fn new(git: &'a Git, rev: &'a str) -> Self {
        Self {
            git,
            rev,
            deleted: Mutex::new(HashMap::new()),
        }
    }

    fn deleted_methods(&self, commit: &str) -> Result<DeletedMethods, GitError> {
        if let Some(known) = self.deleted.lock().expect("cache").get(commit) {
            return Ok(Arc::clone(known));
        }
        let names = self.git.text(&[
            "diff-tree",
            "-r",
            "--no-renames",
            "--diff-filter=D",
            "--name-only",
            "--no-commit-id",
            commit,
        ])?;
        let paths: Vec<(&str, SourceLang)> = names
            .lines()
            .filter_map(|path| Some((path, SourceLang::of_path(path)?)))
            .collect();
        let specs: Vec<String> = paths
            .iter()
            .map(|(path, _)| format!("{commit}^:{path}"))
            .collect();
        let methods: Vec<(String, Vec<String>)> = paths
            .iter()
            .zip(self.git.blobs(&specs))
            .filter_map(|(&(path, lang), blob)| {
                let text = String::from_utf8_lossy(&blob?).into_owned();
                let mut names: Vec<String> = scan::methods(lang, &text)
                    .into_iter()
                    .map(|m| m.name)
                    .collect();
                names.sort();
                names.dedup();
                Some((path.to_string(), names))
            })
            .collect();
        let methods = Arc::new(methods);
        self.deleted
            .lock()
            .expect("cache")
            .insert(commit.to_string(), Arc::clone(&methods));
        Ok(methods)
    }

    pub fn end_of(&self, method: &MethodRef) -> HistoryEnd {
        let mut end = HistoryEnd {
            method: method.clone(),
            commits: 0,
            last_commit: None,
            ends_at_file_add: false,
            deleted_with_same_name: Vec::new(),
            paths: Vec::new(),
            error: None,
        };
        let result = log_funcname(self.git, self.rev, method).and_then(|history| {
            end.commits = history.len();
            for commit in &history {
                if end.paths.last() != Some(&commit.path) {
                    end.paths.push(commit.path.clone());
                }
            }
            let Some(last) = history.last() else {
                return Ok(());
            };
            end.last_commit = Some(last.sha.clone());
            end.ends_at_file_add = last.previous_path.is_none();
            if end.ends_at_file_add {
                end.deleted_with_same_name = self
                    .deleted_methods(&last.sha)?
                    .iter()
                    .filter(|(_, names)| names.binary_search(&method.name).is_ok())
                    .map(|(path, _)| path.clone())
                    .collect();
            }
            Ok(())
        });
        if let Err(error) = result {
            end.error = Some(error.stderr.trim().to_string());
        }
        end
    }
}

/// Item 3: «Обрывы истории».
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryBreaks {
    /// Methods of the revision addressable by `git log -L :<func>:<file>`.
    pub swift_methods: usize,
    pub sampled_methods: usize,
    /// `git log -L` failed for the method.
    pub log_failed: usize,
    /// The history ends at the commit that added the file.
    pub ends_at_file_add: usize,
    pub ends_at_file_add_share: f64,
    /// … and that commit deleted a file with a method of the same name.
    pub breaks: usize,
    pub break_share: f64,
    /// Breaks where a deleted file with the method is Objective-C.
    pub breaks_from_objc: usize,
    pub breaks_from_swift_only: usize,
    /// Median number of commits in a `git log -L` history.
    pub median_commits: usize,
}

pub fn history_ends(
    git: &Git,
    rev: &str,
    refs: &[MethodRef],
    methods: usize,
    seed: u64,
) -> Vec<HistoryEnd> {
    let mut picked = rng::sample(refs.len(), methods, seed);
    picked.sort_unstable();
    let sampled: Vec<&MethodRef> = picked.into_iter().map(|i| &refs[i]).collect();
    let finder = BreakFinder::new(git, rev);
    par_map(&sampled, |method| finder.end_of(method))
}

pub fn history_breaks(population: usize, ends: &[HistoryEnd]) -> HistoryBreaks {
    let at_add = ends.iter().filter(|end| end.ends_at_file_add).count();
    let breaks = ends.iter().filter(|end| end.is_break()).count();
    let from_objc = ends
        .iter()
        .filter(|end| end.is_break() && end.from_objc())
        .count();
    let mut lengths: Vec<usize> = ends.iter().map(|end| end.commits).collect();
    lengths.sort_unstable();
    HistoryBreaks {
        swift_methods: population,
        sampled_methods: ends.len(),
        log_failed: ends.iter().filter(|end| end.error.is_some()).count(),
        ends_at_file_add: at_add,
        ends_at_file_add_share: share(at_add, ends.len()),
        breaks,
        break_share: share(breaks, ends.len()),
        breaks_from_objc: from_objc,
        breaks_from_swift_only: breaks - from_objc,
        median_commits: lengths.get(lengths.len() / 2).copied().unwrap_or(0),
    }
}

/// Whether `keyword` occurs in `message` at the start of a word, ignoring
/// case: `fix` matches «Fixes» and «fixed», not «prefix».
pub fn has_keyword(message: &str, keyword: &str) -> bool {
    let message = message.to_lowercase();
    let keyword = keyword.to_lowercase();
    message.match_indices(&keyword).any(|(at, _)| {
        message[..at]
            .chars()
            .next_back()
            .is_none_or(|before| !before.is_alphanumeric())
    })
}

/// Item 4: «Грубая доля коммитов-багфиксов по ключевым словам».
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BugfixShare {
    /// Commits that are not merges: the base of the share.
    pub commits: usize,
    /// A keyword occurs in the subject or the body.
    pub bugfix_commits: usize,
    pub bugfix_share: f64,
    /// A keyword occurs in the subject line.
    pub bugfix_by_subject: usize,
    pub bugfix_by_subject_share: f64,
    /// Commits per keyword (subject or body); a commit can match several.
    pub by_keyword: BTreeMap<String, usize>,
}

pub fn bugfix_share(history: &[CommitRecord], keywords: &[&str]) -> BugfixShare {
    let commits: Vec<&CommitRecord> = history.iter().filter(|c| !c.is_merge()).collect();
    let mut by_keyword: BTreeMap<String, usize> =
        keywords.iter().map(|k| ((*k).to_string(), 0)).collect();
    let (mut any, mut by_subject) = (0, 0);
    for commit in &commits {
        let mut matched = false;
        for keyword in keywords {
            if has_keyword(&commit.message, keyword) {
                matched = true;
                *by_keyword.get_mut(*keyword).expect("keyword is in the map") += 1;
            }
        }
        any += usize::from(matched);
        by_subject += usize::from(
            keywords
                .iter()
                .any(|keyword| has_keyword(commit.subject(), keyword)),
        );
    }
    BugfixShare {
        commits: commits.len(),
        bugfix_commits: any,
        bugfix_share: share(any, commits.len()),
        bugfix_by_subject: by_subject,
        bugfix_by_subject_share: share(by_subject, commits.len()),
        by_keyword,
    }
}

/// The repository a measurement ran on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MeasuredRepo {
    pub name: String,
    pub url: String,
    /// The pinned commit: every number is computed at this revision.
    pub sha: String,
}

/// `params` of `eval/results/E1-measure.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MeasureParams {
    pub repo: MeasuredRepo,
    pub git_version: String,
    pub seed: u64,
    pub huge_commit_files: u32,
    pub blame_lines: usize,
    pub mass_commit_files: u32,
    pub break_methods: usize,
    pub swift_xfuncname: String,
    pub bugfix_keywords: Vec<String>,
}

/// `metrics` of `eval/results/E1-measure.json`: items 1–4 of the stage.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MeasureMetrics {
    pub repo_stats: RepoStats,
    pub blame_noise: BlameNoise,
    pub history_breaks: HistoryBreaks,
    pub bugfix_share: BugfixShare,
}

/// Sizes and thresholds of the measurement; the defaults are the numbers of
/// docs/TZ.md.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureConfig {
    pub seed: u64,
    pub huge_commit_files: u32,
    pub blame_lines: usize,
    pub mass_commit_files: u32,
    pub break_methods: usize,
}

impl Default for MeasureConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            huge_commit_files: 1000,
            blame_lines: 500,
            mass_commit_files: 200,
            break_methods: 200,
        }
    }
}

/// Runs the four measurements on `repo.sha`.
pub fn measure(
    git: &Git,
    repo: MeasuredRepo,
    config: &MeasureConfig,
) -> Result<(MeasureParams, MeasureMetrics), GitError> {
    let rev = repo.sha.clone();
    let history = commits(git, &rev)?;
    let sources = swift_sources(git, &rev)?;
    let refs = swift_method_refs(&sources);
    let ends = history_ends(git, &rev, &refs, config.break_methods, config.seed);
    let metrics = MeasureMetrics {
        repo_stats: repo_stats(git, &rev, &history, config.huge_commit_files)?,
        blame_noise: blame_noise(
            git,
            &rev,
            &sources,
            &history,
            config.blame_lines,
            config.mass_commit_files,
            config.seed,
        )?,
        history_breaks: history_breaks(refs.len(), &ends),
        bugfix_share: bugfix_share(&history, &BUGFIX_KEYWORDS),
    };
    let params = MeasureParams {
        repo,
        git_version: git.version(),
        seed: config.seed,
        huge_commit_files: config.huge_commit_files,
        blame_lines: config.blame_lines,
        mass_commit_files: config.mass_commit_files,
        break_methods: config.break_methods,
        swift_xfuncname: crate::gitx::SWIFT_XFUNCNAME.to_string(),
        bugfix_keywords: BUGFIX_KEYWORDS.iter().map(ToString::to_string).collect(),
    };
    Ok((params, metrics))
}
