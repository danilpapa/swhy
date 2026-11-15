//! Candidate collection for the method oracle (docs/TZ.md, E1, «Эталон
//! истории методов», item 3): the union of `git log -L` and `git log
//! --follow`, plus merge commits compared with every parent.
//!
//! For every candidate commit the text of the method before and after is
//! extracted with the lexical scanner. Commits that leave the text and the
//! path untouched are dropped mechanically; everything else becomes a
//! [`Step`] that an annotator reviews and classifies. When the method is
//! absent before a commit, the trace proposes where it came from (a renamed
//! method of the same file, a method of a deleted or modified file, an
//! Objective-C method) and continues there. Every proposal is shown to the
//! annotator together with the diff; the annotator can cut the trace at any
//! commit.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::{
    gitx::{Git, GitError},
    scan::{self, Method, SourceLang},
};

/// A method at one revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Located {
    pub path: String,
    pub key: String,
    /// Declaration line and last line, 1-based.
    pub lines: (u32, u32),
    #[serde(skip)]
    pub text: String,
}

/// Why a step is in the review sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    /// The method is absent before the commit and no origin was found.
    Introduced,
    /// Same file, same key, the text differs.
    Changed,
    /// The file was renamed by the commit (the text may differ too).
    FileRenamed,
    /// The method had another key in the same file before the commit.
    Renamed,
    /// The method was in another file before the commit.
    MovedFromFile,
    /// The method was an Objective-C method before the commit.
    CrossLanguage,
    /// A merge commit whose text differs from every parent.
    Merge,
}

/// What a mechanical comparison of the two texts says. A hint, not a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hint {
    /// Byte-identical.
    Identical,
    /// Equal after removing whitespace.
    WhitespaceOnly,
    /// Equal after removing whitespace and comments.
    CommentsOnly,
    Other,
}

/// One candidate commit that touched the method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub commit: String,
    pub date: String,
    pub subject: String,
    /// The parent the method was compared with; `None` for a root commit.
    pub parent: Option<String>,
    pub kind: StepKind,
    pub after: Located,
    pub before: Option<Located>,
    /// Where the candidate came from: `follow`, `log-L`, `merge`.
    pub sources: BTreeSet<String>,
    pub hint: Hint,
    /// Similarity of the two texts (Dice over lines), 0–1, two decimals.
    pub similarity: f64,
    /// Changed lines: `-` before, `+` after.
    pub diff: String,
    /// Other origins that were considered: (similarity, path, key).
    pub alternatives: Vec<(f64, String, String)>,
}

/// Numbers of candidates by source, over all segments of a trace.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateCounts {
    /// Non-merge commits of `git log --follow`.
    pub follow: usize,
    /// Commits of `git log -L <start>,<end>:<file>`.
    pub log_l: usize,
    /// Merge commits of `git rev-list --merges --full-history`.
    pub merges: usize,
    /// Distinct commits in the union.
    pub union: usize,
    /// Candidates of the tool itself; 0 until E4 exists.
    pub tool: usize,
}

/// The trace of one method: steps newest first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trace {
    pub start: Located,
    pub steps: Vec<Step>,
    pub candidates: CandidateCounts,
}

struct Candidate {
    sha: String,
    parents: Vec<String>,
    date: String,
    subject: String,
    path: String,
    previous_path: Option<String>,
    sources: BTreeSet<String>,
}

/// Dice coefficient over trimmed non-empty lines (multiset), two decimals.
pub fn similarity(a: &str, b: &str) -> f64 {
    let lines = |text: &str| -> Vec<String> {
        let mut lines: Vec<String> = text
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
        lines.sort();
        lines
    };
    let (a, b) = (lines(a), lines(b));
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let (mut i, mut j, mut common) = (0, 0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                common += 1;
                i += 1;
                j += 1;
            }
        }
    }
    (2.0 * f64::from(common) / (a.len() + b.len()) as f64 * 100.0).round() / 100.0
}

/// Jaccard over identifier words, for texts in different languages.
pub fn word_similarity(a: &str, b: &str) -> f64 {
    let words = |text: &str| -> BTreeSet<String> {
        text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|word| word.len() > 2 && !word.chars().all(|c| c.is_ascii_digit()))
            .map(str::to_lowercase)
            .collect()
    };
    let (a, b) = (words(a), words(b));
    let union = a.union(&b).count();
    if union == 0 {
        return 0.0;
    }
    (a.intersection(&b).count() as f64 / union as f64 * 100.0).round() / 100.0
}

fn strip(lang: SourceLang, text: &str, comments: bool) -> String {
    let bytes = if comments {
        scan::mask_comments(lang, text)
    } else {
        text.as_bytes().to_vec()
    };
    String::from_utf8_lossy(&bytes)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// Mechanical comparison of two versions of a method.
pub fn hint(lang: SourceLang, before: &str, after: &str) -> Hint {
    if before == after {
        Hint::Identical
    } else if strip(lang, before, false) == strip(lang, after, false) {
        Hint::WhitespaceOnly
    } else if strip(lang, before, true) == strip(lang, after, true) {
        Hint::CommentsOnly
    } else {
        Hint::Other
    }
}

/// Changed lines of `before` → `after` (longest common subsequence).
pub fn line_diff(before: &str, after: &str) -> String {
    let a: Vec<&str> = before.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    let mut lcs = vec![vec![0u32; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = String::new();
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            i += 1;
            j += 1;
        } else if i < a.len() && (j == b.len() || lcs[i + 1][j] >= lcs[i][j + 1]) {
            out.push_str(&format!("-{}\n", a[i]));
            i += 1;
        } else {
            out.push_str(&format!("+{}\n", b[j]));
            j += 1;
        }
    }
    out
}

/// Lowercased letters of a Swift signature or an ObjC selector, without
/// `with`: `load(id:completion:)` and `-loadWithId:completion:` both give
/// `loadidcompletion`. Used only to rank cross-language origin proposals.
fn name_letters(signature: &str) -> String {
    let letters: String = signature
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase();
    letters.replace("with", "")
}

/// Top-level directory of a path: the module (target) in Signal-iOS.
pub fn module_of(path: &str) -> &str {
    path.split('/').next().unwrap_or(path)
}

/// Collects candidates and builds traces.
pub struct Tracer<'a> {
    git: &'a Git,
    blobs: HashMap<String, Option<String>>,
    order: HashMap<String, usize>,
    /// Commits at which the trace must stop (annotator's cut): the method is
    /// treated as introduced there.
    pub cuts: BTreeSet<String>,
}

impl<'a> Tracer<'a> {
    pub fn new(git: &'a Git) -> Self {
        Self {
            git,
            blobs: HashMap::new(),
            order: HashMap::new(),
            cuts: BTreeSet::new(),
        }
    }

    fn prefetch(&mut self, specs: Vec<String>) {
        let mut missing: Vec<String> = specs
            .into_iter()
            .filter(|spec| !self.blobs.contains_key(spec))
            .collect();
        missing.sort();
        missing.dedup();
        let blobs = self.git.blobs(&missing);
        for (spec, blob) in missing.into_iter().zip(blobs) {
            let text = blob.map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
            self.blobs.insert(spec, text);
        }
    }

    fn source(&mut self, rev: &str, path: &str) -> Option<String> {
        let spec = format!("{rev}:{path}");
        self.prefetch(vec![spec.clone()]);
        self.blobs[&spec].clone()
    }

    fn methods(&mut self, rev: &str, path: &str) -> Vec<(Method, String)> {
        let Some(lang) = SourceLang::of_path(path) else {
            return Vec::new();
        };
        let Some(source) = self.source(rev, path) else {
            return Vec::new();
        };
        scan::methods(lang, &source)
            .into_iter()
            .filter(|method| method.has_body)
            .map(|method| {
                let text = method.text(&source);
                (method, text)
            })
            .collect()
    }

    /// The method with one of `keys` (earlier keys win) in `rev:path`.
    pub fn locate(&mut self, rev: &str, path: &str, keys: &[String]) -> Option<Located> {
        let methods = self.methods(rev, path);
        keys.iter().find_map(|key| {
            methods
                .iter()
                .find(|(method, _)| &method.key() == key)
                .map(|(method, text)| Located {
                    path: path.to_string(),
                    key: key.clone(),
                    lines: (method.decl_line, method.end_line),
                    text: text.clone(),
                })
        })
    }

    fn follow(&self, rev: &str, path: &str) -> Result<Vec<Candidate>, GitError> {
        let log = self.git.text(&[
            "log",
            "--follow",
            "-M",
            "--name-status",
            "--format=%x01%H%x09%P%x09%cs%x09%s",
            rev,
            "--",
            path,
        ])?;
        Ok(log
            .split('\u{1}')
            .skip(1)
            .filter_map(|record| {
                let mut lines = record.lines();
                let header: Vec<&str> = lines.next()?.splitn(4, '\t').collect();
                let status: Vec<&str> = lines.find(|line| !line.is_empty())?.split('\t').collect();
                let (path, previous_path) = match status.as_slice() {
                    [code, old, new] if code.starts_with('R') || code.starts_with('C') => {
                        ((*new).to_string(), Some((*old).to_string()))
                    }
                    ["A", path] => ((*path).to_string(), None),
                    [_, path] => ((*path).to_string(), Some((*path).to_string())),
                    _ => return None,
                };
                Some(Candidate {
                    sha: (*header.first()?).to_string(),
                    parents: header
                        .get(1)?
                        .split_whitespace()
                        .map(str::to_string)
                        .collect(),
                    date: (*header.get(2)?).to_string(),
                    subject: header.get(3).unwrap_or(&"").to_string(),
                    path,
                    previous_path,
                    sources: BTreeSet::from(["follow".to_string()]),
                })
            })
            .collect())
    }

    fn log_l(&self, rev: &str, at: &Located) -> Result<Vec<Candidate>, GitError> {
        let range = format!("{},{}:{}", at.lines.0, at.lines.1, at.path);
        let log = self.git.text(&[
            "log",
            "--format=%x01%H%x09%P%x09%cs%x09%s",
            "-L",
            &range,
            rev,
        ])?;
        Ok(log
            .split('\u{1}')
            .skip(1)
            .filter_map(|record| {
                let mut lines = record.lines();
                let header: Vec<&str> = lines.next()?.splitn(4, '\t').collect();
                let (mut before, mut after) = (None, None);
                for line in lines {
                    if let Some(path) = line.strip_prefix("--- ") {
                        before = path.strip_prefix("a/").map(str::to_string);
                    } else if let Some(path) = line.strip_prefix("+++ b/") {
                        after = Some(path.to_string());
                        break;
                    }
                }
                Some(Candidate {
                    sha: (*header.first()?).to_string(),
                    parents: header
                        .get(1)?
                        .split_whitespace()
                        .map(str::to_string)
                        .collect(),
                    date: (*header.get(2)?).to_string(),
                    subject: header.get(3).unwrap_or(&"").to_string(),
                    path: after?,
                    previous_path: before,
                    sources: BTreeSet::from(["log-L".to_string()]),
                })
            })
            .collect())
    }

    fn position(&mut self, rev: &str, sha: &str) -> usize {
        if self.order.is_empty() {
            let list = self.git.text(&["rev-list", rev]).unwrap_or_default();
            self.order = list
                .lines()
                .enumerate()
                .map(|(i, sha)| (sha.to_string(), i))
                .collect();
        }
        self.order.get(sha).copied().unwrap_or(usize::MAX)
    }

    fn step(
        candidate: &Candidate,
        parent: Option<&String>,
        kind: StepKind,
        after: Located,
        before: Option<Located>,
        alternatives: Vec<(f64, String, String)>,
    ) -> Step {
        let lang = SourceLang::of_path(&after.path).unwrap_or(SourceLang::Swift);
        let before_text = before.as_ref().map_or("", |b| b.text.as_str());
        let cross = kind == StepKind::CrossLanguage;
        Step {
            commit: candidate.sha.clone(),
            date: candidate.date.clone(),
            subject: candidate.subject.clone(),
            parent: parent.cloned(),
            kind,
            sources: candidate.sources.clone(),
            hint: if before.is_some() && !cross {
                hint(lang, before_text, &after.text)
            } else {
                Hint::Other
            },
            similarity: if cross {
                word_similarity(before_text, &after.text)
            } else {
                similarity(before_text, &after.text)
            },
            diff: if cross || before.is_none() {
                String::new()
            } else {
                line_diff(before_text, &after.text)
            },
            after,
            before,
            alternatives,
        }
    }

    /// Where the method `after` of `commit` was in `parent`: a method that
    /// the commit removed from the same file or from another changed file.
    fn origin(
        &mut self,
        commit: &str,
        parent: &str,
        after: &Located,
        previous_path: Option<&str>,
    ) -> Result<Vec<(f64, Located)>, GitError> {
        let status = self.git.text(&[
            "diff-tree",
            "-r",
            "--no-renames",
            "--name-status",
            "--no-commit-id",
            commit,
        ])?;
        let mut files: Vec<(String, bool)> = status
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .filter(|(code, path)| *code != "A" && SourceLang::of_path(path).is_some())
            .map(|(code, path)| (path.to_string(), code == "D"))
            .collect();
        if let Some(path) = previous_path
            && !files.iter().any(|(file, _)| file == path)
        {
            files.push((path.to_string(), false));
        }
        let name = after
            .key
            .rsplit('.')
            .next()
            .unwrap_or("")
            .split(['(', ':'])
            .next()
            .unwrap_or("");
        let name = name.trim_start_matches(['-', '+']).to_string();
        self.prefetch(
            files
                .iter()
                .map(|(path, _)| format!("{parent}:{path}"))
                .collect(),
        );
        self.prefetch(
            files
                .iter()
                .filter(|(_, deleted)| !deleted)
                .map(|(path, _)| format!("{commit}:{path}"))
                .collect(),
        );
        let after_lang = SourceLang::of_path(&after.path);
        let after_letters = name_letters(after.key.rsplit('.').next().unwrap_or(""));
        let mut found = Vec::new();
        for (path, deleted) in files {
            let still_there: BTreeSet<String> = if deleted {
                BTreeSet::new()
            } else {
                self.methods(commit, &path)
                    .iter()
                    .map(|(m, _)| m.key())
                    .collect()
            };
            for (method, text) in self.methods(parent, &path) {
                let key = method.key();
                if still_there.contains(&key) {
                    continue;
                }
                let same_lang = Some(method.lang) == after_lang;
                let letters = name_letters(&method.signature);
                let same_name = if same_lang {
                    method.name == name
                } else {
                    letters == after_letters
                        || method.name.eq_ignore_ascii_case(&name)
                        || letters.starts_with(&name.to_lowercase())
                };
                let score = if same_lang {
                    similarity(&text, &after.text)
                } else {
                    word_similarity(&text, &after.text)
                };
                let accepted = if same_lang {
                    score >= 0.6 || (same_name && score >= 0.3) || key == after.key
                } else {
                    same_name && score >= 0.15
                };
                if accepted {
                    let rank = score
                        + if key == after.key || same_name {
                            1.0
                        } else {
                            0.0
                        };
                    found.push((
                        rank,
                        Located {
                            path: path.clone(),
                            key,
                            lines: (method.decl_line, method.end_line),
                            text,
                        },
                    ));
                }
            }
        }
        found.sort_by(|a, b| {
            b.0.total_cmp(&a.0)
                .then_with(|| (&a.1.path, &a.1.key).cmp(&(&b.1.path, &b.1.key)))
        });
        Ok(found)
    }

    /// Traces the method `keys[0]` of `rev:path` back to its introduction.
    pub fn trace(&mut self, rev: &str, path: &str, key: &str) -> Result<Trace, String> {
        let start = self
            .locate(rev, path, &[key.to_string()])
            .ok_or_else(|| format!("{key} not found in {rev}:{path}"))?;
        let mut steps = Vec::new();
        let mut counts = CandidateCounts::default();
        let mut union = BTreeSet::new();
        let mut segment = Some((rev.to_string(), start.clone()));
        let mut guard = 0;
        while let Some((segment_rev, at)) = segment.take() {
            guard += 1;
            if guard > 12 {
                return Err("more than 12 segments: the trace does not converge".to_string());
            }
            segment = self
                .segment(rev, &segment_rev, &at, &mut steps, &mut counts, &mut union)
                .map_err(|error| error.to_string())?;
        }
        counts.union = union.len();
        Ok(Trace {
            start,
            steps,
            candidates: counts,
        })
    }

    /// One file of the history. Returns where to continue, if the method
    /// came from another file.
    fn segment(
        &mut self,
        top: &str,
        rev: &str,
        at: &Located,
        steps: &mut Vec<Step>,
        counts: &mut CandidateCounts,
        union: &mut BTreeSet<String>,
    ) -> Result<Option<(String, Located)>, GitError> {
        let follow = self.follow(rev, &at.path)?;
        let log_l = self.log_l(rev, at)?;
        counts.follow += follow.len();
        counts.log_l += log_l.len();
        let mut candidates: BTreeMap<String, Candidate> = BTreeMap::new();
        for candidate in follow.into_iter().chain(log_l) {
            match candidates.get_mut(&candidate.sha) {
                Some(known) => known.sources.extend(candidate.sources),
                None => {
                    candidates.insert(candidate.sha.clone(), candidate);
                }
            }
        }
        let mut paths: BTreeSet<String> = BTreeSet::from([at.path.clone()]);
        for candidate in candidates.values() {
            paths.insert(candidate.path.clone());
            paths.extend(candidate.previous_path.clone());
        }
        // Merge commits that touch any of the paths.
        let mut args = vec![
            "log",
            "--merges",
            "--full-history",
            "--format=%H%x09%P%x09%cs%x09%s",
            rev,
            "--",
        ];
        args.extend(paths.iter().map(String::as_str));
        let merges = self.git.text(&args)?;
        let merges: Vec<Candidate> = merges
            .lines()
            .filter_map(|line| {
                let fields: Vec<&str> = line.splitn(4, '\t').collect();
                Some(Candidate {
                    sha: (*fields.first()?).to_string(),
                    parents: fields
                        .get(1)?
                        .split_whitespace()
                        .map(str::to_string)
                        .collect(),
                    date: (*fields.get(2)?).to_string(),
                    subject: fields.get(3).unwrap_or(&"").to_string(),
                    path: String::new(),
                    previous_path: None,
                    sources: BTreeSet::from(["merge".to_string()]),
                })
            })
            .collect();
        counts.merges += merges.len();
        union.extend(candidates.keys().cloned());
        union.extend(merges.iter().map(|merge| merge.sha.clone()));

        let mut ordered: Vec<Candidate> = candidates.into_values().collect();
        ordered.sort_by_key(|candidate| self.position(top, &candidate.sha));
        self.prefetch(
            ordered
                .iter()
                .flat_map(|c| {
                    let before = c
                        .parents
                        .first()
                        .zip(c.previous_path.as_ref())
                        .map(|(parent, path)| format!("{parent}:{path}"));
                    [Some(format!("{}:{}", c.sha, c.path)), before]
                })
                .flatten()
                .collect(),
        );

        let mut keys = vec![at.key.clone()];
        let mut next = None;
        let mut end = None;
        for candidate in &ordered {
            if candidate.parents.len() > 1 {
                continue;
            }
            let Some(after) = self.locate(&candidate.sha, &candidate.path, &keys) else {
                continue;
            };
            let parent = candidate.parents.first();
            let before = parent
                .zip(candidate.previous_path.as_ref())
                .and_then(|(parent, path)| self.locate(parent, path, &keys));
            if let Some(before) = before {
                let renamed = before.path != after.path;
                if before.text != after.text || renamed {
                    let kind = if renamed {
                        StepKind::FileRenamed
                    } else {
                        StepKind::Changed
                    };
                    steps.push(Self::step(
                        candidate,
                        parent,
                        kind,
                        after,
                        Some(before),
                        Vec::new(),
                    ));
                }
                continue;
            }
            // The method is absent before this commit.
            let origins = match parent {
                Some(parent) if !self.cuts.contains(&candidate.sha) => self.origin(
                    &candidate.sha,
                    parent,
                    &after,
                    candidate.previous_path.as_deref(),
                )?,
                _ => Vec::new(),
            };
            let alternatives: Vec<(f64, String, String)> = origins
                .iter()
                .skip(1)
                .take(3)
                .map(|(rank, origin)| (*rank, origin.path.clone(), origin.key.clone()))
                .collect();
            let Some((_, origin)) = origins.into_iter().next() else {
                steps.push(Self::step(
                    candidate,
                    parent,
                    StepKind::Introduced,
                    after,
                    None,
                    Vec::new(),
                ));
                end = Some(candidate.sha.clone());
                break;
            };
            let same_file = candidate.previous_path.as_deref() == Some(origin.path.as_str());
            let cross = SourceLang::of_path(&origin.path) != SourceLang::of_path(&after.path);
            let kind = if cross {
                StepKind::CrossLanguage
            } else if same_file {
                StepKind::Renamed
            } else {
                StepKind::MovedFromFile
            };
            steps.push(Self::step(
                candidate,
                parent,
                kind,
                after,
                Some(origin.clone()),
                alternatives,
            ));
            if same_file && !cross {
                keys.insert(0, origin.key);
            } else {
                next = parent.map(|parent| (parent.clone(), origin));
                end = Some(candidate.sha.clone());
                break;
            }
        }

        // Merges newer than the end of the segment: changed against every parent.
        let end_position = end
            .as_ref()
            .map_or(usize::MAX, |sha| self.position(top, sha));
        for merge in merges {
            if self.position(top, &merge.sha) >= end_position {
                continue;
            }
            let locate_any = |tracer: &mut Self, rev: &str| {
                paths
                    .iter()
                    .rev()
                    .find_map(|path| tracer.locate(rev, path, &keys))
            };
            let Some(after) = locate_any(self, &merge.sha) else {
                continue;
            };
            let parents: Vec<Option<Located>> = merge
                .parents
                .iter()
                .map(|parent| locate_any(self, parent))
                .collect();
            let differs_from_all = parents
                .iter()
                .all(|parent| parent.as_ref().is_none_or(|p| p.text != after.text));
            if differs_from_all && parents.iter().any(Option::is_some) {
                let (index, before) = parents
                    .into_iter()
                    .enumerate()
                    .find_map(|(i, parent)| parent.map(|p| (i, p)))
                    .expect("one parent has the method");
                let parent = merge.parents.get(index);
                steps.push(Self::step(
                    &merge,
                    parent,
                    StepKind::Merge,
                    after,
                    Some(before),
                    Vec::new(),
                ));
            }
        }
        steps.sort_by_key(|step| self.position(top, &step.commit));
        steps.dedup_by(|a, b| a.commit == b.commit);
        Ok(next)
    }
}

/// The first `max_lines` lines of `text`; the rest is replaced by a count.
fn head(text: &str, max_lines: usize) -> String {
    let total = text.lines().count();
    if total <= max_lines {
        return text.to_string();
    }
    let mut out: String = text
        .lines()
        .take(max_lines)
        .flat_map(|line| [line, "\n"])
        .collect();
    out.push_str(&format!("    … {} more lines\n", total - max_lines));
    out
}

/// The review sheet of a trace: what an annotator reads. Whole method texts
/// (an introduction, both sides of a rewrite) are cut to `max_lines` lines;
/// diffs are never cut.
pub fn sheet(id: &str, trace: &Trace, max_lines: usize) -> String {
    let c = &trace.candidates;
    let mut out = format!(
        "== {id} {} @ {} | candidates: follow={} log-L={} merges={} union={} | steps={}\n",
        trace.start.key,
        trace.start.path,
        c.follow,
        c.log_l,
        c.merges,
        c.union,
        trace.steps.len()
    );
    for (number, step) in trace.steps.iter().enumerate() {
        let sources: Vec<&str> = step.sources.iter().map(String::as_str).collect();
        out.push_str(&format!(
            "[{}] {} {} {:?} hint={:?} sim={} src={} \"{}\"\n",
            number + 1,
            &step.commit[..10],
            step.date,
            step.kind,
            step.hint,
            step.similarity,
            sources.join("+"),
            step.subject
        ));
        if let Some(before) = &step.before
            && (before.path != step.after.path || before.key != step.after.key)
        {
            out.push_str(&format!(
                "    from {} @ {}\n    to   {} @ {}\n",
                before.key, before.path, step.after.key, step.after.path
            ));
        }
        for (rank, path, key) in &step.alternatives {
            out.push_str(&format!("    alt {rank:.2} {key} @ {path}\n"));
        }
        match (&step.before, step.kind) {
            (Some(before), StepKind::CrossLanguage) => {
                out.push_str(&format!(
                    "--- before\n{}+++ after\n{}",
                    head(&before.text, max_lines),
                    head(&step.after.text, max_lines)
                ));
            }
            (None, _) => {
                out.push_str(&format!(
                    "+++ introduced\n{}",
                    head(&step.after.text, max_lines)
                ));
            }
            _ => out.push_str(&step.diff),
        }
    }
    out
}
