//! E1, «Замер проблемы», on the fixture `fx-measure`. Every expected number
//! is derived by hand from the scenario in `fixtures/fx-measure.sh`.

use std::collections::BTreeMap;

use swhy_eval::{
    gitx::Git,
    measure::{
        self, BUGFIX_KEYWORDS, BlameOrigin, BreakFinder, HugeCommit, MeasureConfig, MeasuredRepo,
        MethodRef, ObjcFiles,
    },
};

/// Commits of `fx-measure`, oldest first (see the header of the script).
const ADD_GREETER: &str = "8498749";
const REINDENT: &str = "06d8dc9";
const HEADERS: &str = "d8d870c";
const CONVERT_OLDMATH: &str = "0c8ed60";
const MOVE_GREETER: &str = "80ad589";
const ADD_FORMAT_AMOUNT: &str = "7a1d762";

fn repo() -> Git {
    Git::open(&fixtures::build("fx-measure"))
}

fn full(git: &Git, short: &str) -> String {
    git.resolve(short).unwrap()
}

/// Thresholds scaled down to the fixture: a mass commit changes more than 3
/// files, a huge one more than 4; the samples take every line and method.
fn config() -> MeasureConfig {
    MeasureConfig {
        seed: 7,
        huge_commit_files: 4,
        blame_lines: 500,
        mass_commit_files: 3,
        break_methods: 200,
    }
}

fn measured(git: &Git, rev: &str) -> MeasuredRepo {
    MeasuredRepo {
        name: "fx-measure".to_string(),
        url: String::new(),
        sha: full(git, rev),
    }
}

#[test]
fn item_1_repository_statistics() {
    let git = repo();
    let (_, metrics) = measure::measure(&git, measured(&git, "HEAD"), &config()).unwrap();
    let stats = metrics.repo_stats;

    assert_eq!(stats.commits, 10);
    assert_eq!(stats.merge_commits, 0);
    assert_eq!(stats.first_commit_date, "2020-03-01");
    assert_eq!(stats.last_commit_date, "2024-01-15");
    assert_eq!(
        (stats.first_year.as_str(), stats.last_year.as_str()),
        ("2020", "2024")
    );
    assert_eq!(
        stats.commits_by_year,
        BTreeMap::from([
            ("2020".to_string(), 3),
            ("2021".to_string(), 2),
            ("2022".to_string(), 2),
            ("2023".to_string(), 2),
            ("2024".to_string(), 1),
        ])
    );
    // App/Config.swift, Core/Greeting/Greeter.swift, Core/Logger.swift,
    // Core/Math.swift and README.md.
    assert_eq!(stats.files, 5);
    assert_eq!(stats.swift_files, 4);
    assert_eq!(
        stats.objc_files,
        ObjcFiles {
            total: 0,
            h: 0,
            m: 0,
            mm: 0
        }
    );
    // Only «Add copyright headers» changes more than 4 files.
    assert_eq!(stats.huge_commits, 1);
    assert_eq!(
        stats.huge_commit_list,
        [HugeCommit {
            sha: full(&git, HEADERS),
            date: "2021-09-01".to_string(),
            files: 5,
            subject: "Add copyright headers".to_string(),
        }]
    );
}

#[test]
fn item_1_counts_objc_files_while_they_exist() {
    let git = repo();
    let rev = full(&git, HEADERS);
    let history = measure::commits(&git, &rev).unwrap();
    let stats = measure::repo_stats(&git, &rev, &history, 1000).unwrap();

    assert_eq!(stats.commits, 5);
    assert_eq!(stats.swift_files, 2);
    assert_eq!(
        stats.objc_files,
        ObjcFiles {
            total: 2,
            h: 1,
            m: 1,
            mm: 0
        }
    );
    assert_eq!(stats.huge_commits, 0);
}

#[test]
fn item_2_blame_noise() {
    let git = repo();
    let (params, metrics) = measure::measure(&git, measured(&git, "HEAD"), &config()).unwrap();
    let noise = metrics.blame_noise;

    assert_eq!(params.blame_lines, 500);
    assert_eq!(params.mass_commit_files, 3);
    // 11 + 36 + 8 + 16 lines in the four Swift files; the sample is all of them.
    assert_eq!(noise.swift_files, 4);
    assert_eq!(noise.swift_lines, 71);
    assert_eq!(noise.sampled_lines, 71);
    // Blank lines: Config 3, 7; Greeter 3, 8, 11, 15, 22, 26, 30; Logger 2;
    // Math 3, 8, 13.
    assert_eq!(noise.sampled_blank_lines, 13);

    // `git blame -w`: only line 1 of App/Config.swift comes from the mass
    // commit «Add copyright headers» (5 files > 3). The reindented line 9 of
    // that file is attributed to the commit that wrote it, not to «Reindent».
    assert_eq!(noise.blame_w.mass_commit, 1);
    assert_eq!(noise.blame_w.whitespace_only, 0);
    assert_eq!(noise.blame_w.technical, 1);
    assert_eq!(noise.blame_w.technical_share, 0.0141);
    // Config: 2 commits; Greeter: 1; Logger: 2; Math: 2.
    assert_eq!(noise.blame_w.distinct_commits, 7);

    // `-M -C -C` follows the copied body of describe (Greeter lines 32–36)
    // into App/Greeter.swift of the first commit, which Config already uses.
    assert_eq!(noise.blame_w_m_c_c.mass_commit, 1);
    assert_eq!(noise.blame_w_m_c_c.whitespace_only, 0);
    assert_eq!(noise.blame_w_m_c_c.technical, 1);
    assert_eq!(noise.blame_w_m_c_c.technical_share, 0.0141);
    assert_eq!(noise.blame_w_m_c_c.distinct_commits, 7);
    assert_eq!(noise.modes_differ, 5);
    assert_eq!(noise.modes_differ_share, 0.0704);
}

#[test]
fn item_2_blame_origins_and_whitespace_only_commits() {
    let git = repo();
    let head = full(&git, "HEAD");

    let plain = measure::blame_file(&git, &head, "App/Config.swift", &["-w"]).unwrap();
    assert_eq!(plain.len(), 11);
    assert_eq!(plain[0].sha, full(&git, HEADERS));
    assert_eq!(plain[8].sha, full(&git, ADD_GREETER));
    assert_eq!(plain[8].previous, None);

    let copies = measure::blame_file(
        &git,
        &head,
        "Core/Greeting/Greeter.swift",
        &["-w", "-M", "-C", "-C"],
    )
    .unwrap();
    assert_eq!(copies.len(), 36);
    assert_eq!(copies[30].sha, full(&git, MOVE_GREETER));
    assert_eq!(copies[30].path, "Core/Greeting/Greeter.swift");
    assert_eq!(copies[31].sha, full(&git, ADD_GREETER));
    assert_eq!(copies[31].path, "App/Greeter.swift");

    // «Reindent farewell and isValid» changes only whitespace in Config.swift.
    let reindent = BlameOrigin {
        sha: full(&git, REINDENT),
        path: "App/Config.swift".to_string(),
        previous: Some((full(&git, "06d8dc9^"), "App/Config.swift".to_string())),
    };
    assert!(measure::is_whitespace_only(&git, &reindent).unwrap());
    // «Add copyright headers» adds a line.
    assert!(!measure::is_whitespace_only(&git, &plain[0]).unwrap());
    // A commit that added the file has no previous version.
    assert!(!measure::is_whitespace_only(&git, &plain[8]).unwrap());
}

#[test]
fn item_3_history_breaks() {
    let git = repo();
    let (params, metrics) = measure::measure(&git, measured(&git, "HEAD"), &config()).unwrap();
    let breaks = metrics.history_breaks;

    assert_eq!(params.break_methods, 200);
    // isValid; greet, farewell, welcome, describe; log; addValue, reset,
    // formatAmount. `init` is not a `func`.
    assert_eq!(breaks.swift_methods, 9);
    assert_eq!(breaks.sampled_methods, 9);
    assert_eq!(breaks.log_failed, 0);
    // All but formatAmount, which was added to an existing file.
    assert_eq!(breaks.ends_at_file_add, 8);
    assert_eq!(breaks.ends_at_file_add_share, 0.8889);
    // greet, farewell, describe (deleted App/Greeter.swift) and addValue,
    // reset (deleted Legacy/OldMath.{h,m}). welcome, isValid and log have no
    // deleted namesake.
    assert_eq!(breaks.breaks, 5);
    assert_eq!(breaks.break_share, 0.5556);
    assert_eq!(breaks.breaks_from_objc, 2);
    assert_eq!(breaks.breaks_from_swift_only, 3);
    // Histories: isValid 2, greet 1, farewell 1, welcome 1, describe 1,
    // log 2, addValue 1, reset 2, formatAmount 1.
    assert_eq!(breaks.median_commits, 1);
}

#[test]
fn item_3_end_of_single_histories() {
    let git = repo();
    let head = full(&git, "HEAD");
    let finder = BreakFinder::new(&git, &head);
    let method = |path: &str, name: &str, line: u32| MethodRef {
        path: path.to_string(),
        name: name.to_string(),
        line,
    };

    let reset = finder.end_of(&method("Core/Math.swift", "reset", 9));
    assert_eq!(reset.error, None);
    assert_eq!(reset.commits, 2);
    assert_eq!(reset.last_commit, Some(full(&git, CONVERT_OLDMATH)));
    assert!(reset.ends_at_file_add);
    assert_eq!(
        reset.deleted_with_same_name,
        ["Legacy/OldMath.h", "Legacy/OldMath.m"]
    );
    assert!(reset.is_break() && reset.from_objc());

    let greet = finder.end_of(&method("Core/Greeting/Greeter.swift", "greet", 16));
    assert_eq!(greet.last_commit, Some(full(&git, MOVE_GREETER)));
    assert_eq!(greet.deleted_with_same_name, ["App/Greeter.swift"]);
    assert!(greet.is_break() && !greet.from_objc());

    let welcome = finder.end_of(&method("Core/Greeting/Greeter.swift", "welcome", 27));
    assert!(welcome.ends_at_file_add);
    assert!(welcome.deleted_with_same_name.is_empty());
    assert!(!welcome.is_break());

    let format_amount = finder.end_of(&method("Core/Math.swift", "formatAmount", 14));
    assert_eq!(
        format_amount.last_commit,
        Some(full(&git, ADD_FORMAT_AMOUNT))
    );
    assert!(!format_amount.ends_at_file_add);
    assert!(!format_amount.is_break());

    let missing = finder.end_of(&method("Core/Math.swift", "noSuchMethod", 1));
    assert_eq!(missing.commits, 0);
    assert!(
        missing
            .error
            .as_deref()
            .is_some_and(|e| e.contains("no match"))
    );
    assert!(!missing.is_break());
}

#[test]
fn item_3_stock_git_cannot_address_an_indented_swift_method() {
    // The reason for the `xfuncname` in `params`: without it the command of
    // the spec fails on every method inside a type.
    let repo = fixtures::build("fx-measure");
    let error = std::process::Command::new("git")
        .current_dir(&repo)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .args(["log", "-s", "-L", ":reset:Core/Math.swift"])
        .output()
        .unwrap();
    assert!(!error.status.success());
    assert!(String::from_utf8_lossy(&error.stderr).contains("no match"));
}

#[test]
fn item_4_bugfix_share() {
    let git = repo();
    let (params, metrics) = measure::measure(&git, measured(&git, "HEAD"), &config()).unwrap();
    let bugfix = metrics.bugfix_share;

    assert_eq!(params.bugfix_keywords, BUGFIX_KEYWORDS);
    assert_eq!(bugfix.commits, 10);
    // «Fix crash in greet for an empty name» and «Skip empty messages in the
    // logger (regression after the rewrite)».
    assert_eq!(bugfix.bugfix_commits, 2);
    assert_eq!(bugfix.bugfix_share, 0.2);
    assert_eq!(bugfix.bugfix_by_subject, 2);
    assert_eq!(bugfix.by_keyword["fix"], 1);
    assert_eq!(bugfix.by_keyword["crash"], 1);
    assert_eq!(bugfix.by_keyword["regression"], 1);
    assert_eq!(bugfix.by_keyword["bug"], 0);
    assert_eq!(bugfix.by_keyword.len(), BUGFIX_KEYWORDS.len());
}

#[test]
fn keywords_match_at_the_start_of_a_word_ignoring_case() {
    assert!(measure::has_keyword("Fixes the layout", "fix"));
    assert!(measure::has_keyword("layout: FIXED", "fix"));
    assert!(measure::has_keyword("Hotfix for login", "hotfix"));
    assert!(!measure::has_keyword("Add a prefix to keys", "fix"));
    assert!(!measure::has_keyword("Add debug logging", "bug"));
    assert!(measure::has_keyword("guard against nil", "nil"));
    assert!(measure::has_keyword(
        "Avoid EXC_BAD_ACCESS",
        "EXC_BAD_ACCESS"
    ));
    assert!(measure::has_keyword("Исправлен краш", "исправ"));
    assert!(measure::has_keyword("Исправлен краш", "краш"));
}

#[test]
fn the_measurement_is_deterministic_and_reports_its_parameters() {
    let git = repo();
    let repo = measured(&git, "HEAD");
    let first = measure::measure(&git, repo.clone(), &config()).unwrap();
    let second = measure::measure(&git, repo.clone(), &config()).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_string(&first.1).unwrap(),
        serde_json::to_string(&second.1).unwrap()
    );
    let params = first.0;
    assert_eq!(params.repo, repo);
    assert_eq!(params.seed, 7);
    assert_eq!(params.huge_commit_files, 4);
    assert_eq!(
        params.swift_xfuncname,
        "^[ \t]*(.*[^A-Za-z0-9_])?func[ \t]+.*$"
    );
    assert!(!params.git_version.is_empty());

    // The defaults are the numbers of the spec.
    let defaults = MeasureConfig::default();
    assert_eq!(defaults.huge_commit_files, 1000);
    assert_eq!(defaults.blame_lines, 500);
    assert_eq!(defaults.mass_commit_files, 200);
    assert_eq!(defaults.break_methods, 200);
}

#[test]
fn a_sample_smaller_than_the_population_has_the_requested_size() {
    let git = repo();
    let small = MeasureConfig {
        blame_lines: 20,
        break_methods: 4,
        ..config()
    };
    let (_, metrics) = measure::measure(&git, measured(&git, "HEAD"), &small).unwrap();
    assert_eq!(metrics.blame_noise.sampled_lines, 20);
    assert_eq!(metrics.blame_noise.swift_lines, 71);
    assert_eq!(metrics.history_breaks.sampled_methods, 4);
    assert_eq!(metrics.history_breaks.swift_methods, 9);
}
