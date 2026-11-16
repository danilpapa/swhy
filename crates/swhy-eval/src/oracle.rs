//! The method-history oracle of E1 (docs/TZ.md, «Эталон истории методов»):
//! selection pool, annotation files, validation and Cohen's kappa.
//!
//! The annotation source is `eval/oracle/labels/<repo>.tsv`: one block per
//! method with a class for every reviewed commit. `oracle-build` re-traces
//! every method, checks that each step of the trace has a label and writes
//! `eval/oracle/methods/<repo>-<n>.json`.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::{
    gitx::Git,
    measure::{self, HistoryEnd},
    scan::{self, SourceLang},
    trace::{CandidateCounts, Step, StepKind, Trace, Tracer, module_of},
};

pub const SCHEMA: &str = "swhy-oracle-method/1";

/// A reviewed commit that did not change the method.
pub const NOT_CHANGING: &str = "NotChanging";

/// Classes of a changing commit (docs/oracle-guide.md).
pub const CLASSES: [&str; 8] = [
    "Introduced",
    "FormatOnly",
    "DocOnly",
    "RenameOnly",
    "SignatureChanged: attributes",
    "Refactoring",
    "Semantic",
    "MoveOnly",
];

/// Minimum number of changing commits besides the introduction.
pub const MIN_CHANGING_COMMITS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodAt {
    pub path: String,
    pub key: String,
}

/// One changing commit of a method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleCommit {
    pub commit: String,
    pub date: String,
    /// The parent the method was compared with.
    pub parent: Option<String>,
    /// The method after the commit.
    pub path: String,
    pub key: String,
    /// The method before the commit, when its path or key differs.
    pub from: Option<MethodAt>,
    pub class: String,
    /// `file`, `module` or `cross_language` when the commit moved the method.
    #[serde(rename = "move")]
    pub move_kind: Option<String>,
    /// Candidate sources that proposed the commit.
    pub sources: Vec<String>,
    pub note: Option<String>,
}

/// `eval/oracle/methods/<repo>-<n>.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleMethod {
    pub schema: String,
    pub id: String,
    pub repo: String,
    /// The pinned revision the method is taken from.
    pub rev: String,
    /// `train` (threshold tuning) or `test` (not to be used before E17).
    pub split: String,
    pub annotator: String,
    pub method: MethodAt,
    pub lines: (u32, u32),
    pub candidates: CandidateCounts,
    /// The commit that introduced the method (the first entry of `history`).
    pub introduced: String,
    pub objc_origin: bool,
    pub module_move: bool,
    /// Changing commits, oldest first.
    pub history: Vec<OracleCommit>,
    /// Commits that were reviewed and judged not to change the method.
    pub not_changing: Vec<String>,
}

impl OracleMethod {
    /// Changing commits besides the introduction.
    pub fn changing_commits(&self) -> usize {
        self.history
            .iter()
            .filter(|commit| commit.class != "Introduced")
            .count()
    }
}

/// One method block of the labels file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelBlock {
    pub id: String,
    pub split: String,
    pub path: String,
    pub key: String,
    /// Commit prefix → (class, note).
    pub labels: Vec<(String, String, Option<String>)>,
    /// Commit prefixes where the trace is cut: the method is introduced there.
    pub cuts: Vec<String>,
}

/// Short class names accepted in the labels file.
fn class_of(code: &str) -> Option<&'static str> {
    Some(match code {
        "I" | "Introduced" => "Introduced",
        "F" | "FormatOnly" => "FormatOnly",
        "D" | "DocOnly" => "DocOnly",
        "R" | "RenameOnly" => "RenameOnly",
        "A" | "SignatureChanged: attributes" => "SignatureChanged: attributes",
        "X" | "Refactoring" => "Refactoring",
        "S" | "Semantic" => "Semantic",
        "M" | "MoveOnly" => "MoveOnly",
        "N" | "NotChanging" => NOT_CHANGING,
        _ => return None,
    })
}

/// Parses a labels file. Lines: `M <id> <split> <path> <key>`, `C <commit>
/// <class> [note]`, `X <commit>`; fields are separated by tabs; `#` starts a
/// comment line.
pub fn parse_labels(text: &str) -> Result<Vec<LabelBlock>, String> {
    let mut blocks: Vec<LabelBlock> = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let error = |message: &str| format!("labels line {}: {message}", number + 1);
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.as_slice() {
            ["M", id, split, path, key] => {
                if !matches!(*split, "train" | "test") {
                    return Err(error("split must be train or test"));
                }
                blocks.push(LabelBlock {
                    id: (*id).to_string(),
                    split: (*split).to_string(),
                    path: (*path).to_string(),
                    key: (*key).to_string(),
                    labels: Vec::new(),
                    cuts: Vec::new(),
                });
            }
            ["C", commit, class, rest @ ..] => {
                let class = class_of(class).ok_or_else(|| error("unknown class"))?;
                let block = blocks.last_mut().ok_or_else(|| error("C before M"))?;
                let note = rest.first().map(|note| (*note).to_string());
                block
                    .labels
                    .push(((*commit).to_string(), class.to_string(), note));
            }
            ["X", commit] => {
                let block = blocks.last_mut().ok_or_else(|| error("X before M"))?;
                block.cuts.push((*commit).to_string());
            }
            _ => return Err(error("expected M, C or X with tab-separated fields")),
        }
    }
    Ok(blocks)
}

fn move_kind(step: &Step) -> Option<&'static str> {
    let before = step.before.as_ref()?;
    if step.kind == StepKind::CrossLanguage {
        Some("cross_language")
    } else if before.path == step.after.path {
        None
    } else if module_of(&before.path) == module_of(&step.after.path) {
        Some("file")
    } else {
        Some("module")
    }
}

/// Resolves the commit prefixes of `block.cuts` and traces the method.
pub fn trace_block(git: &Git, rev: &str, block: &LabelBlock) -> Result<Trace, String> {
    let mut tracer = Tracer::new(git);
    for cut in &block.cuts {
        tracer
            .cuts
            .insert(git.resolve(cut).map_err(|error| error.to_string())?);
    }
    tracer.trace(rev, &block.path, &block.key)
}

/// Builds the oracle file of one method from its trace and labels.
pub fn build_method(
    repo: &str,
    rev: &str,
    annotator: &str,
    block: &LabelBlock,
    trace: &Trace,
) -> Result<OracleMethod, String> {
    let error = |message: String| format!("{}: {message}", block.id);
    let mut used = BTreeSet::new();
    let mut history = Vec::new();
    let mut not_changing = Vec::new();
    for step in trace.steps.iter().rev() {
        let matching: Vec<&(String, String, Option<String>)> = block
            .labels
            .iter()
            .filter(|(prefix, _, _)| step.commit.starts_with(prefix))
            .collect();
        let (class, note) = match matching.as_slice() {
            [(prefix, class, note)] => {
                used.insert(prefix.clone());
                (class.clone(), note.clone())
            }
            [] => return Err(error(format!("step {} has no label", &step.commit[..10]))),
            _ => {
                return Err(error(format!(
                    "step {} has several labels",
                    &step.commit[..10]
                )));
            }
        };
        if (class == "Introduced") != (step.kind == StepKind::Introduced) {
            return Err(error(format!(
                "{}: class Introduced must match the end of the trace (cut the trace with X)",
                &step.commit[..10]
            )));
        }
        if class == NOT_CHANGING {
            not_changing.push(step.commit.clone());
            continue;
        }
        let from = step
            .before
            .as_ref()
            .filter(|before| before.path != step.after.path || before.key != step.after.key)
            .map(|before| MethodAt {
                path: before.path.clone(),
                key: before.key.clone(),
            });
        history.push(OracleCommit {
            commit: step.commit.clone(),
            date: step.date.clone(),
            parent: step.parent.clone(),
            path: step.after.path.clone(),
            key: step.after.key.clone(),
            from,
            class,
            move_kind: move_kind(step).map(str::to_string),
            sources: step.sources.iter().cloned().collect(),
            note,
        });
    }
    if let Some((prefix, _, _)) = block
        .labels
        .iter()
        .find(|(prefix, _, _)| !used.contains(prefix))
    {
        return Err(error(format!(
            "label {prefix} matches no step of the trace"
        )));
    }
    let introduced = history
        .first()
        .filter(|first| first.class == "Introduced")
        .map(|first| first.commit.clone())
        .ok_or_else(|| error("the trace does not end with Introduced".to_string()))?;
    if history
        .iter()
        .filter(|commit| commit.class == "Introduced")
        .count()
        != 1
    {
        return Err(error("more than one Introduced".to_string()));
    }
    let crosses_module = |commit: &OracleCommit| {
        commit
            .from
            .as_ref()
            .is_some_and(|from| module_of(&from.path) != module_of(&commit.path))
    };
    Ok(OracleMethod {
        schema: SCHEMA.to_string(),
        id: block.id.clone(),
        repo: repo.to_string(),
        rev: rev.to_string(),
        split: block.split.clone(),
        annotator: annotator.to_string(),
        method: MethodAt {
            path: trace.start.path.clone(),
            key: trace.start.key.clone(),
        },
        lines: trace.start.lines,
        candidates: trace.candidates.clone(),
        introduced,
        objc_origin: history
            .iter()
            .any(|c| c.move_kind.as_deref() == Some("cross_language")),
        module_move: history.iter().any(crosses_module),
        history,
        not_changing,
    })
}

/// Pretty JSON with a trailing newline.
pub fn to_json(value: &impl Serialize) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("serializable");
    text.push('\n');
    text
}

/// Loads `<dir>/*.json` sorted by file name.
pub fn load_dir(dir: &Path) -> Result<Vec<OracleMethod>, String> {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .map_err(|error| format!("{}: {error}", dir.display()))?
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|path| {
            let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let method: OracleMethod =
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("");
            if method.id != stem {
                return Err(format!(
                    "{}: id {} does not match the file name",
                    path.display(),
                    method.id
                ));
            }
            Ok(method)
        })
        .collect()
}

/// Agreement of two annotators over the same items.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Agreement {
    pub items: usize,
    pub agreed: usize,
    pub observed: f64,
    pub expected: f64,
    /// Cohen's kappa; `None` when it is undefined (expected agreement 1).
    pub kappa: Option<f64>,
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

/// Cohen's kappa for pairs of labels: κ = (pₒ − pₑ) / (1 − pₑ).
pub fn cohen_kappa(pairs: &[(String, String)]) -> Agreement {
    let n = pairs.len() as f64;
    let agreed = pairs.iter().filter(|(a, b)| a == b).count();
    let (mut first, mut second): (BTreeMap<&str, f64>, BTreeMap<&str, f64>) = Default::default();
    for (a, b) in pairs {
        *first.entry(a).or_default() += 1.0;
        *second.entry(b).or_default() += 1.0;
    }
    let observed = if pairs.is_empty() {
        0.0
    } else {
        agreed as f64 / n
    };
    let expected: f64 = first
        .iter()
        .map(|(label, count)| count / n * second.get(label).copied().unwrap_or(0.0) / n)
        .sum();
    let kappa = (!pairs.is_empty() && (1.0 - expected).abs() > 1e-12)
        .then(|| round4((observed - expected) / (1.0 - expected)));
    Agreement {
        items: pairs.len(),
        agreed,
        observed: round4(observed),
        expected: round4(expected),
        kappa,
    }
}

/// Labels of one annotator for one method: commit → class.
fn labels_of(method: &OracleMethod) -> BTreeMap<&str, &str> {
    method
        .history
        .iter()
        .map(|commit| (commit.commit.as_str(), commit.class.as_str()))
        .chain(
            method
                .not_changing
                .iter()
                .map(|commit| (commit.as_str(), NOT_CHANGING)),
        )
        .collect()
}

/// Agreement of the second annotator with the first.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecondAnnotator {
    /// «Второй разметчик проверяет 30% методов».
    pub methods_required: usize,
    pub methods: usize,
    pub method_ids: Vec<String>,
    pub annotators: Vec<String>,
    /// Did the commit change the method: changed / not changed.
    pub changed: Option<Agreement>,
    /// The class of the commit, `NotChanging` included.
    pub class: Option<Agreement>,
}

pub fn second_annotator(
    first: &[OracleMethod],
    second: &[OracleMethod],
) -> Result<SecondAnnotator, String> {
    let mut class_pairs = Vec::new();
    let mut ids = Vec::new();
    let mut annotators = BTreeSet::new();
    for theirs in second {
        let ours = first
            .iter()
            .find(|method| method.id == theirs.id)
            .ok_or_else(|| format!("second annotation {} has no first annotation", theirs.id))?;
        if ours.annotator == theirs.annotator {
            return Err(format!(
                "{}: the second annotator is the first one",
                theirs.id
            ));
        }
        annotators.insert(theirs.annotator.clone());
        ids.push(theirs.id.clone());
        let (a, b) = (labels_of(ours), labels_of(theirs));
        let commits: BTreeSet<&str> = a.keys().chain(b.keys()).copied().collect();
        for commit in commits {
            let label = |labels: &BTreeMap<&str, &str>| {
                labels
                    .get(commit)
                    .copied()
                    .unwrap_or(NOT_CHANGING)
                    .to_string()
            };
            class_pairs.push((label(&a), label(&b)));
        }
    }
    let binary = |label: &String| if label == NOT_CHANGING { "no" } else { "yes" }.to_string();
    let changed_pairs: Vec<(String, String)> = class_pairs
        .iter()
        .map(|(a, b)| (binary(a), binary(b)))
        .collect();
    let present = !second.is_empty();
    Ok(SecondAnnotator {
        methods_required: (first.len() * 3).div_ceil(10),
        methods: second.len(),
        method_ids: ids,
        annotators: annotators.into_iter().collect(),
        changed: present.then(|| cohen_kappa(&changed_pairs)),
        class: present.then(|| cohen_kappa(&class_pairs)),
    })
}

/// `params` of the oracle check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckParams {
    pub methods_dir: String,
    pub second_dir: String,
    pub min_changing_commits: usize,
}

/// `metrics` of the oracle check: the numbers of the acceptance criterion.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CheckMetrics {
    pub methods: usize,
    pub repos: BTreeMap<String, usize>,
    pub train: usize,
    pub test: usize,
    pub annotators: Vec<String>,
    /// Smallest number of changing commits (besides the introduction).
    pub min_changing_commits: usize,
    pub methods_below_min: Vec<String>,
    pub with_module_move: usize,
    pub with_objc_origin: usize,
    pub with_both: usize,
    pub changing_commits: usize,
    pub commits_by_class: BTreeMap<String, usize>,
    pub commits_by_move: BTreeMap<String, usize>,
    pub reviewed_not_changing: usize,
    pub candidates_union: usize,
    pub second_annotator: SecondAnnotator,
    /// Requirements of docs/TZ.md that do not hold; empty when all hold.
    pub violations: Vec<String>,
}

pub fn check(first: &[OracleMethod], second: &[OracleMethod]) -> Result<CheckMetrics, String> {
    let mut repos = BTreeMap::new();
    let mut by_class = BTreeMap::new();
    let mut by_move = BTreeMap::new();
    let mut violations = Vec::new();
    for method in first {
        *repos.entry(method.repo.clone()).or_default() += 1;
        if method.schema != SCHEMA {
            violations.push(format!("{}: schema {}", method.id, method.schema));
        }
        if method.history.first().map(|c| c.commit.as_str()) != Some(method.introduced.as_str()) {
            violations.push(format!("{}: introduced is not the first commit", method.id));
        }
        for commit in &method.history {
            if !CLASSES.contains(&commit.class.as_str()) {
                violations.push(format!("{}: class {}", method.id, commit.class));
            }
            *by_class.entry(commit.class.clone()).or_default() += 1;
            if let Some(kind) = &commit.move_kind {
                *by_move.entry(kind.clone()).or_default() += 1;
            }
        }
    }
    let count =
        |predicate: &dyn Fn(&OracleMethod) -> bool| first.iter().filter(|m| predicate(m)).count();
    let below: Vec<String> = first
        .iter()
        .filter(|method| method.changing_commits() < MIN_CHANGING_COMMITS)
        .map(|method| method.id.clone())
        .collect();
    let metrics_second = second_annotator(first, second)?;
    let (train, test) = (
        count(&|m| m.split == "train"),
        count(&|m| m.split == "test"),
    );
    let (moves, objc) = (count(&|m| m.module_move), count(&|m| m.objc_origin));
    let mut require = |ok: bool, message: String| {
        if !ok {
            violations.push(message);
        }
    };
    require(
        first.len() == 50,
        format!("50 methods required, found {}", first.len()),
    );
    require(
        train == 25 && test == 25,
        format!("split 25/25 required, found {train}/{test}"),
    );
    require(
        below.is_empty(),
        format!(
            "methods with fewer than 3 changing commits: {}",
            below.len()
        ),
    );
    require(
        moves >= 10,
        format!("10 methods with a module move required, found {moves}"),
    );
    require(
        objc >= 10,
        format!("10 methods with ObjC origin required, found {objc}"),
    );
    require(
        metrics_second.methods >= metrics_second.methods_required,
        format!(
            "second annotator: {} methods required, found {}",
            metrics_second.methods_required, metrics_second.methods
        ),
    );
    require(
        metrics_second
            .class
            .as_ref()
            .is_some_and(|agreement| agreement.kappa.is_some()),
        "Cohen's kappa is not computed".to_string(),
    );
    let annotators: BTreeSet<String> = first.iter().map(|m| m.annotator.clone()).collect();
    Ok(CheckMetrics {
        methods: first.len(),
        repos,
        train,
        test,
        annotators: annotators.into_iter().collect(),
        min_changing_commits: first
            .iter()
            .map(OracleMethod::changing_commits)
            .min()
            .unwrap_or(0),
        methods_below_min: below,
        with_module_move: moves,
        with_objc_origin: objc,
        with_both: count(&|m| m.module_move && m.objc_origin),
        changing_commits: first.iter().map(|m| m.history.len()).sum(),
        commits_by_class: by_class,
        commits_by_move: by_move,
        reviewed_not_changing: first.iter().map(|m| m.not_changing.len()).sum(),
        candidates_union: first.iter().map(|m| m.candidates.union).sum(),
        second_annotator: metrics_second,
        violations,
    })
}

/// A method of the selection pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolEntry {
    pub path: String,
    pub key: String,
    pub lines: u32,
    /// Commits in `git log -L :<func>:<file>`.
    pub log_commits: usize,
    /// The `git log -L` history breaks at a deleted Objective-C file.
    pub objc_break: bool,
    /// The history went through, or broke between, two top-level directories.
    pub module_change: bool,
}

/// Paths that are not eligible for the oracle: tests, generated and
/// third-party code.
pub fn is_excluded_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    [
        "tests/",
        "/test/",
        "thirdparty/",
        "/generated/",
        "pods/",
        ".pb.swift",
        ".generated.swift",
    ]
    .iter()
    .any(|part| lower.contains(part))
}

/// Samples `size` methods of `rev` and describes their `git log -L` history.
/// The order is the order of the draw: selection takes the first eligible.
pub fn pool(git: &Git, rev: &str, size: usize, seed: u64) -> Result<Vec<PoolEntry>, String> {
    let sources: Vec<(String, String)> = measure::swift_sources(git, rev)
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|(path, _)| !is_excluded_path(path))
        .collect();
    let refs = measure::swift_method_refs(&sources);
    let order = crate::rng::sample(refs.len(), size, seed);
    let picked: Vec<&measure::MethodRef> = order.iter().map(|&i| &refs[i]).collect();
    let finder = measure::BreakFinder::new(git, rev);
    let ends: Vec<HistoryEnd> = crate::gitx::par_map(&picked, |method| finder.end_of(method));
    let texts: BTreeMap<&str, &str> = sources
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    Ok(ends
        .into_iter()
        .filter_map(|end| {
            let source = texts.get(end.method.path.as_str())?;
            let method = scan::methods(SourceLang::Swift, source)
                .into_iter()
                .find(|m| m.decl_line == end.method.line && m.has_body)?;
            let mut modules: Vec<&str> = end.paths.iter().map(|path| module_of(path)).collect();
            modules.extend(
                end.deleted_with_same_name
                    .iter()
                    .map(|path| module_of(path)),
            );
            modules.sort_unstable();
            modules.dedup();
            Some(PoolEntry {
                key: method.key(),
                lines: method.end_line + 1 - method.start_line,
                log_commits: end.commits,
                objc_break: end.is_break() && end.from_objc(),
                module_change: modules.len() > 1,
                path: end.method.path,
            })
        })
        .collect())
}

/// Selection quotas and limits (docs/oracle-guide.md, «Отбор методов»).
pub const QUOTA_OBJC: usize = 12;
pub const QUOTA_MODULE: usize = 12;
pub const QUOTA_GENERAL: usize = 26;
/// Longest method, in lines, that is reviewed by hand.
pub const MAX_METHOD_LINES: u32 = 80;
/// Most changing commits, besides the introduction, reviewed by hand.
pub const MAX_CHANGING_COMMITS: usize = 12;

/// The stratum a method is selected for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stratum {
    ObjcOrigin,
    ModuleMove,
    General,
}

/// A pool method with its trace.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Screened {
    pub stratum: Stratum,
    pub entry: PoolEntry,
    /// Steps of the trace besides the introduction.
    pub changing_steps: usize,
    pub objc_origin: bool,
    pub module_move: bool,
    /// Why the method is not eligible; `None` when it is.
    pub rejected: Option<String>,
    #[serde(skip)]
    pub trace: Option<Trace>,
}

/// Traces the first pool methods of every stratum and marks the eligible.
pub fn screen(git: &Git, rev: &str, pool: &[PoolEntry]) -> Vec<Screened> {
    let fits = |entry: &&PoolEntry| entry.lines <= MAX_METHOD_LINES;
    let objc = pool.iter().filter(fits).filter(|e| e.objc_break).take(40);
    let module = pool
        .iter()
        .filter(fits)
        .filter(|e| e.module_change && !e.objc_break && (2..=15).contains(&e.log_commits))
        .take(40);
    let general = pool
        .iter()
        .filter(fits)
        .filter(|e| !e.module_change && !e.objc_break && (4..=15).contains(&e.log_commits))
        .take(80);
    let candidates: Vec<(Stratum, &PoolEntry)> = objc
        .map(|e| (Stratum::ObjcOrigin, e))
        .chain(module.map(|e| (Stratum::ModuleMove, e)))
        .chain(general.map(|e| (Stratum::General, e)))
        .collect();
    crate::gitx::par_map(&candidates, |&(stratum, entry)| {
        let block = LabelBlock {
            id: String::new(),
            split: "train".to_string(),
            path: entry.path.clone(),
            key: entry.key.clone(),
            labels: Vec::new(),
            cuts: Vec::new(),
        };
        let duplicate_key = git.blob_text(rev, &entry.path).is_some_and(|source| {
            scan::methods(SourceLang::Swift, &source)
                .iter()
                .filter(|m| m.key() == entry.key)
                .count()
                > 1
        });
        let trace = trace_block(git, rev, &block);
        let (mut steps, mut objc_origin, mut module_move) = (0, false, false);
        let mut rejected = None;
        match &trace {
            Err(error) => rejected = Some(error.clone()),
            Ok(trace) => {
                steps = trace
                    .steps
                    .iter()
                    .filter(|s| s.kind != StepKind::Introduced)
                    .count();
                objc_origin = trace
                    .steps
                    .iter()
                    .any(|s| s.kind == StepKind::CrossLanguage);
                module_move = trace.steps.iter().any(|s| {
                    s.before
                        .as_ref()
                        .is_some_and(|b| module_of(&b.path) != module_of(&s.after.path))
                });
                let introduced = trace
                    .steps
                    .iter()
                    .filter(|s| s.kind == StepKind::Introduced)
                    .count();
                if duplicate_key {
                    rejected = Some("two methods with this key in the file".to_string());
                } else if introduced != 1 {
                    rejected = Some(format!("{introduced} introductions in the trace"));
                } else if !(MIN_CHANGING_COMMITS..=MAX_CHANGING_COMMITS).contains(&steps) {
                    rejected = Some(format!("{steps} changing steps"));
                } else if stratum == Stratum::ObjcOrigin && !objc_origin {
                    rejected = Some("the trace has no ObjC step".to_string());
                } else if stratum == Stratum::ModuleMove && !module_move {
                    rejected = Some("the trace has no module move".to_string());
                }
            }
        }
        Screened {
            stratum,
            entry: entry.clone(),
            changing_steps: steps,
            objc_origin,
            module_move,
            rejected,
            trace: trace.ok(),
        }
    })
}

/// The first eligible methods of every stratum, in the order of the draw:
/// (id, split, method). `skip` lists `path\tkey` of methods rejected by the
/// annotator. Splits alternate inside a stratum.
pub fn select<'a>(
    slug: &str,
    screened: &'a [Screened],
    skip: &BTreeSet<String>,
) -> Vec<(String, &'static str, &'a Screened)> {
    let mut selected = Vec::new();
    for (stratum, quota) in [
        (Stratum::ObjcOrigin, QUOTA_OBJC),
        (Stratum::ModuleMove, QUOTA_MODULE),
        (Stratum::General, QUOTA_GENERAL),
    ] {
        let eligible = screened
            .iter()
            .filter(|s| s.stratum == stratum && s.rejected.is_none())
            .filter(|s| !skip.contains(&format!("{}\t{}", s.entry.path, s.entry.key)))
            .take(quota);
        for (index, method) in eligible.enumerate() {
            let id = format!("{slug}-{:02}", selected.len() + 1);
            selected.push((id, if index % 2 == 0 { "train" } else { "test" }, method));
        }
    }
    selected
}

/// A labels file for the selected methods with the mechanical classes
/// filled in and `?` everywhere a decision is needed.
pub fn skeleton(selected: &[(String, &'static str, &Screened)]) -> String {
    let mut out = String::new();
    for (id, split, method) in selected {
        out.push_str(&format!(
            "M\t{id}\t{split}\t{}\t{}\n",
            method.entry.path, method.entry.key
        ));
        for step in method.trace.iter().flat_map(|trace| &trace.steps) {
            let class = match (step.kind, step.hint) {
                (StepKind::Introduced, _) => "I",
                (StepKind::FileRenamed, crate::trace::Hint::Identical) => "M",
                _ => "?",
            };
            out.push_str(&format!("C\t{}\t{class}\n", &step.commit[..10]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(table: &[(&str, &str, usize)]) -> Vec<(String, String)> {
        table
            .iter()
            .flat_map(|&(a, b, n)| std::iter::repeat_n((a.to_string(), b.to_string()), n))
            .collect()
    }

    #[test]
    fn kappa_of_a_two_by_two_table() {
        // 20 + 15 agreements out of 50: pₒ = 0.7; marginals 25/25 and 30/20:
        // pₑ = 0.5·0.6 + 0.5·0.4 = 0.5; κ = 0.2 / 0.5 = 0.4.
        let agreement = cohen_kappa(&pairs(&[
            ("y", "y", 20),
            ("y", "n", 5),
            ("n", "y", 10),
            ("n", "n", 15),
        ]));
        assert_eq!(agreement.items, 50);
        assert_eq!(agreement.agreed, 35);
        assert_eq!(agreement.observed, 0.7);
        assert_eq!(agreement.expected, 0.5);
        assert_eq!(agreement.kappa, Some(0.4));
    }

    #[test]
    fn kappa_of_three_classes() {
        // Diagonal 8 + 6 + 4 of 24: pₒ = 0.75. Rows 10, 8, 6; columns 9, 9, 6:
        // pₑ = (90 + 72 + 36) / 576 = 0.34375; κ = 0.40625 / 0.65625 = 0.6190.
        let agreement = cohen_kappa(&pairs(&[
            ("a", "a", 8),
            ("a", "b", 2),
            ("b", "b", 6),
            ("b", "c", 2),
            ("c", "a", 1),
            ("c", "b", 1),
            ("c", "c", 4),
        ]));
        assert_eq!(agreement.observed, 0.75);
        assert_eq!(agreement.expected, 0.3438);
        assert_eq!(agreement.kappa, Some(0.619));
    }

    #[test]
    fn kappa_edge_cases() {
        assert_eq!(
            cohen_kappa(&pairs(&[("a", "a", 3), ("b", "b", 3)])).kappa,
            Some(1.0)
        );
        // One class only: expected agreement is 1, kappa is undefined.
        assert_eq!(cohen_kappa(&pairs(&[("a", "a", 5)])).kappa, None);
        assert_eq!(cohen_kappa(&[]).kappa, None);
        // Systematic disagreement.
        assert_eq!(
            cohen_kappa(&pairs(&[("a", "b", 5), ("b", "a", 5)])).kappa,
            Some(-1.0)
        );
    }

    #[test]
    fn labels_file_is_parsed_and_validated() {
        let text = "# comment\nM\tx-01\ttrain\tA/B.swift\tswift:B.f()\nC\tabc1234\tS\tnote here\nC\tdef5678\tSignatureChanged: attributes\nX\t0123abc\n\nM\tx-02\ttest\tA/C.swift\tswift:C.g()\nC\t999\tI\n";
        let blocks = parse_labels(text).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].id, "x-01");
        assert_eq!(
            blocks[0].labels,
            [
                (
                    "abc1234".to_string(),
                    "Semantic".to_string(),
                    Some("note here".to_string())
                ),
                (
                    "def5678".to_string(),
                    "SignatureChanged: attributes".to_string(),
                    None
                ),
            ]
        );
        assert_eq!(blocks[0].cuts, ["0123abc"]);
        assert_eq!(
            (blocks[1].split.as_str(), blocks[1].key.as_str()),
            ("test", "swift:C.g()")
        );

        assert!(parse_labels("C\tabc\tS\n").is_err());
        assert!(parse_labels("M\tx\tdev\tp\tk\n").is_err());
        assert!(parse_labels("M\tx\ttrain\tp\tk\nC\tabc\tWeird\n").is_err());
    }

    fn screened(stratum: Stratum, name: &str, rejected: Option<&str>) -> Screened {
        Screened {
            stratum,
            entry: PoolEntry {
                path: format!("{name}.swift"),
                key: format!("swift:T.{name}()"),
                lines: 10,
                log_commits: 5,
                objc_break: stratum == Stratum::ObjcOrigin,
                module_change: stratum == Stratum::ModuleMove,
            },
            changing_steps: 4,
            objc_origin: stratum == Stratum::ObjcOrigin,
            module_move: stratum == Stratum::ModuleMove,
            rejected: rejected.map(str::to_string),
            trace: None,
        }
    }

    #[test]
    fn selection_takes_the_first_eligible_of_each_stratum_and_alternates_splits() {
        let pool = [
            screened(Stratum::General, "g1", None),
            screened(Stratum::ObjcOrigin, "o1", None),
            screened(Stratum::ObjcOrigin, "o2", Some("2 changing steps")),
            screened(Stratum::ObjcOrigin, "o3", None),
            screened(Stratum::ModuleMove, "m1", None),
            screened(Stratum::General, "g2", None),
            screened(Stratum::General, "g3", None),
        ];
        let ids = |skip: &[&str]| -> Vec<(String, &str, String)> {
            let skip = skip.iter().map(ToString::to_string).collect();
            select("x", &pool, &skip)
                .into_iter()
                .map(|(id, split, method)| (id, split, method.entry.path.clone()))
                .collect()
        };
        let row =
            |id: &str, split: &'static str, path: &str| (id.to_string(), split, path.to_string());
        assert_eq!(
            ids(&[]),
            [
                row("x-01", "train", "o1.swift"),
                row("x-02", "test", "o3.swift"),
                row("x-03", "train", "m1.swift"),
                row("x-04", "train", "g1.swift"),
                row("x-05", "test", "g2.swift"),
                row("x-06", "train", "g3.swift"),
            ]
        );
        // A method rejected by the annotator is replaced by the next one.
        assert_eq!(
            ids(&["g1.swift\tswift:T.g1()"])[3..],
            [
                row("x-04", "train", "g2.swift"),
                row("x-05", "test", "g3.swift")
            ]
        );
        let all = BTreeSet::new();
        assert_eq!(
            skeleton(&select("x", &pool, &all)[..2]),
            "M\tx-01\ttrain\to1.swift\tswift:T.o1()\nM\tx-02\ttest\to3.swift\tswift:T.o3()\n"
        );
        assert_eq!((QUOTA_OBJC, QUOTA_MODULE, QUOTA_GENERAL), (12, 12, 26));
    }

    #[test]
    fn excluded_paths() {
        assert!(is_excluded_path("SignalServiceKitTests/Foo.swift"));
        assert!(is_excluded_path("Signal/test/Foo.swift"));
        assert!(is_excluded_path("Signal/Emoji/Generated/Emoji+Name.swift"));
        assert!(is_excluded_path(
            "SignalServiceKit/protobuf/Backup.pb.swift"
        ));
        assert!(!is_excluded_path("SignalServiceKit/Messages/Foo.swift"));
    }
}
