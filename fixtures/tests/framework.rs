//! E0: the fixture framework. Acceptance criterion of docs/TZ.md:
//! «`fixtures::build` дважды даёт одинаковые SHA».

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

/// HEAD of `fx-rename`. Changes only when `fixtures/fx-rename.sh` or
/// `fixtures/lib.sh` change.
const FX_RENAME_HEAD: &str = "917c1cb2dda0bdab223ff8077f935b2b56fb65cf";
/// HEAD of `fx-smoke`.
const FX_SMOKE_HEAD: &str = "bad3f5edc9c7d08b03097303aa1f0b0e1fd39712";

/// A directory under `target/fixtures/.fresh/`, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::SeqCst);
        let path = fixtures::root()
            .join(".fresh")
            .join(format!("{tag}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        Scratch(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn build_twice_gives_the_same_sha() {
    let first = fixtures::build("fx-rename");
    let first_sha = fixtures::head_sha(&first);
    let second = fixtures::build("fx-rename");
    let second_sha = fixtures::head_sha(&second);

    assert_eq!(first, second);
    assert_eq!(first_sha, second_sha);
    assert_eq!(first_sha, FX_RENAME_HEAD);
}

#[test]
fn build_puts_the_repository_under_target_fixtures() {
    let repo = fixtures::build("fx-smoke");

    assert_eq!(repo, fixtures::root().join("fx-smoke"));
    assert!(
        repo.ends_with("fixtures/fx-smoke"),
        "unexpected path {}",
        repo.display()
    );
    if std::env::var_os("CARGO_TARGET_DIR").is_none() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        assert_eq!(repo, workspace.join("target/fixtures/fx-smoke"));
    }
    assert!(repo.join(".git").is_dir());
    assert_eq!(fixtures::head_sha(&repo), FX_SMOKE_HEAD);
}

#[test]
fn independent_builds_give_identical_repositories() {
    let names = fixtures::names();
    assert!(names.contains(&"fx-rename".to_string()));
    assert!(names.contains(&"fx-smoke".to_string()));

    for name in names {
        let a = Scratch::new(&format!("{name}-a"));
        let b = Scratch::new(&format!("{name}-b"));
        fixtures::build_fresh(&name, a.path());
        fixtures::build_fresh(&name, b.path());

        let commits_a = fixtures::commit_shas(a.path());
        let commits_b = fixtures::commit_shas(b.path());
        assert!(!commits_a.is_empty(), "{name}: no commits");
        assert_eq!(commits_a, commits_b, "{name}: commit SHAs differ");
        assert_eq!(
            fixtures::refs(a.path()),
            fixtures::refs(b.path()),
            "{name}: refs differ"
        );
        assert_eq!(
            fixtures::head_sha(a.path()),
            fixtures::head_sha(b.path()),
            "{name}: HEAD differs"
        );
    }
}

#[test]
fn fresh_build_matches_the_pinned_shas() {
    for (name, head, commits) in [
        ("fx-rename", FX_RENAME_HEAD, 7),
        ("fx-smoke", FX_SMOKE_HEAD, 6),
    ] {
        let dir = Scratch::new(&format!("{name}-pinned"));
        fixtures::build_fresh(name, dir.path());
        assert_eq!(fixtures::head_sha(dir.path()), head, "{name}");
        assert_eq!(fixtures::commit_shas(dir.path()).len(), commits, "{name}");
    }
}

#[test]
fn machine_git_environment_does_not_change_shas() {
    let home = Scratch::new("hostile-home");
    fs::create_dir_all(home.path()).unwrap();
    let gitconfig = home.path().join(".gitconfig");
    fs::write(
        &gitconfig,
        "[user]\n\tname = Mallory\n\temail = mallory@example.com\n\
         [commit]\n\tgpgsign = true\n\
         [core]\n\tautocrlf = true\n\
         [init]\n\tdefaultBranch = trunk\n",
    )
    .unwrap();
    let home_str = home.path().to_str().unwrap();
    let gitconfig_str = gitconfig.to_str().unwrap();

    let dir = Scratch::new("hostile-repo");
    fixtures::build_fresh_with_env(
        "fx-rename",
        dir.path(),
        &[
            ("HOME", home_str),
            ("XDG_CONFIG_HOME", home_str),
            ("GIT_CONFIG_GLOBAL", gitconfig_str),
            ("GIT_AUTHOR_NAME", "Mallory"),
            ("GIT_AUTHOR_EMAIL", "mallory@example.com"),
            ("GIT_COMMITTER_NAME", "Mallory"),
            ("GIT_COMMITTER_EMAIL", "mallory@example.com"),
            ("GIT_AUTHOR_DATE", "2001-01-01T00:00:00+00:00"),
            ("GIT_COMMITTER_DATE", "2001-01-01T00:00:00+00:00"),
            ("TZ", "Asia/Tokyo"),
            ("LC_ALL", "ru_RU.UTF-8"),
        ],
    );

    assert_eq!(fixtures::head_sha(dir.path()), FX_RENAME_HEAD);
    assert_eq!(
        fixtures::git(dir.path(), &["symbolic-ref", "--short", "HEAD"]),
        "main"
    );
}

#[test]
fn author_email_and_dates_are_fixed() {
    let repo = fixtures::build("fx-smoke");
    let log = fixtures::git(
        &repo,
        &[
            "log",
            "--topo-order",
            "--reverse",
            "--format=%an|%ae|%aI|%cn|%ce|%cI|%s",
        ],
    );
    let who = "Fixture Author|fixture@swhy.test";
    let expected = [
        format!("{who}|2024-01-10T10:00:00Z|{who}|2024-01-10T10:00:00Z|Add Greeter and Counter"),
        format!(
            "{who}|2024-01-11T12:30:00+03:00|{who}|2024-01-11T12:30:00+03:00|Fix empty name, move Counter to Core"
        ),
        format!("{who}|2024-01-12T09:15:00Z|{who}|2024-01-12T09:15:00Z|Remove Counter"),
        format!("{who}|2024-01-14T08:00:00Z|{who}|2024-01-14T08:00:00Z|Add README"),
        format!("{who}|2024-01-13T08:00:00Z|{who}|2024-01-13T08:00:00Z|Add farewell"),
        format!("{who}|2024-01-15T08:00:00Z|{who}|2024-01-15T08:00:00Z|Merge branch 'side'"),
    ];
    let mut actual: Vec<&str> = log.lines().collect();
    let mut expected: Vec<&str> = expected.iter().map(String::as_str).collect();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

#[test]
fn fx_smoke_exercises_write_move_remove_and_merge() {
    let repo = fixtures::build("fx-smoke");

    assert_eq!(
        fixtures::git(&repo, &["ls-files"]),
        "README.md\nSources/App/Farewell.swift\nSources/App/Greeter.swift"
    );
    assert_eq!(
        fs::read_to_string(repo.join("Sources/App/Greeter.swift")).unwrap(),
        "struct Greeter {\n    func greet(_ name: String) -> String {\n        \
         guard !name.isEmpty else { return \"Hello!\" }\n        \
         return \"Hello, \\(name)!\"\n    }\n}\n"
    );
    // Commit 2 is a pure move of Counter.swift (same blob, new path).
    assert_eq!(
        fixtures::git(
            &repo,
            &["show", "--format=", "--name-status", "-M", "HEAD~3"]
        ),
        "M\tSources/App/Greeter.swift\nR100\tSources/App/Legacy/Counter.swift\tSources/Core/Counter.swift"
    );
    assert_eq!(
        fixtures::git(&repo, &["show", "--format=", "--name-status", "HEAD~2"]),
        "D\tSources/Core/Counter.swift"
    );
    // HEAD is a merge commit with two parents.
    assert_eq!(
        fixtures::git(&repo, &["rev-list", "--parents", "-n", "1", "HEAD"])
            .split(' ')
            .count(),
        3
    );
    assert_eq!(fixtures::git(&repo, &["status", "--porcelain"]), "");
}

#[test]
fn fx_rename_has_the_history_shape_listed_for_e4() {
    let repo = fixtures::build("fx-rename");

    assert_eq!(
        fixtures::git(
            &repo,
            &["log", "--first-parent", "--reverse", "--format=%s"]
        ),
        "Add CardViewModel\n\
         Rename refresh to reloadCards\n\
         Use load(for:) label\n\
         Store fetched cards before calling completion\n\
         Merge branch 'extract-reload'\n\
         Fix duplicated cards after reload"
    );
    assert_eq!(
        fixtures::git(&repo, &["log", "--format=%s", "-n", "1", "HEAD~1^2"]),
        "Move reloadCards into an extension"
    );
    assert_eq!(
        fixtures::git(&repo, &["ls-files"]),
        "Cards/CardViewModel+Reload.swift\nCards/CardViewModel.swift"
    );

    let class = fs::read_to_string(repo.join("Cards/CardViewModel.swift")).unwrap();
    let extension = fs::read_to_string(repo.join("Cards/CardViewModel+Reload.swift")).unwrap();
    // Overloads live in the class, the renamed method lives in the extension.
    assert!(class.contains("    func load(for id: String) {\n"));
    assert!(
        class.contains("    func load(id: String, completion: @escaping ([Card]) -> Void) {\n")
    );
    assert!(class.contains("api.fetchCards(id: id) { [weak self] result in\n"));
    assert!(!class.contains("reloadCards"));
    assert!(!class.contains("refresh"));
    assert!(
        extension.starts_with(
            "import Foundation\n\nextension CardViewModel {\n    func reloadCards() {\n"
        )
    );
    assert!(extension.contains("let ids = Set(cards.map { $0.id })\n"));
    assert_eq!(fixtures::git(&repo, &["status", "--porcelain"]), "");
}

#[test]
fn build_runs_the_script_at_most_once_in_a_process() {
    let paths: Vec<PathBuf> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| fixtures::build("fx-smoke")))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    assert!(paths.iter().all(|p| p == &paths[0]));
    // 0 when another test binary of the same run has already built it.
    assert!(fixtures::script_runs("fx-smoke") <= 1);
    let token = fixtures::build_token("fx-smoke").expect("stamp exists after build");
    fixtures::build("fx-smoke");
    assert_eq!(fixtures::build_token("fx-smoke").unwrap(), token);
}

/// Runs `swhy-fixtures build <name>` with its own target directory and run id
/// and returns `(head, build token)`.
fn build_in_process(target_dir: &Path, run_id: &str, name: &str) -> (String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_swhy-fixtures"))
        .args(["build", name])
        .env("CARGO_TARGET_DIR", target_dir)
        .env(fixtures::RUN_ID_ENV, run_id)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "swhy-fixtures failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let field = |key: &str| -> String {
        stdout
            .lines()
            .find_map(|l| l.strip_prefix(key))
            .unwrap_or_else(|| panic!("no {key} in {stdout:?}"))
            .to_string()
    };
    let path = PathBuf::from(field("path="));
    assert_eq!(path, target_dir.join("fixtures").join(name));
    (field("head="), field("build="))
}

#[test]
fn build_runs_the_script_once_per_test_run_across_processes() {
    let target = Scratch::new("once-per-run-target");

    // Four processes of one run start together: the script runs once.
    let same_run: Vec<(String, String)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| scope.spawn(|| build_in_process(target.path(), "run-1", "fx-rename")))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let (head, token) = same_run[0].clone();
    assert_eq!(head, FX_RENAME_HEAD);
    assert!(same_run.iter().all(|r| r == &(head.clone(), token.clone())));

    // A later process of the same run reuses the repository.
    assert_eq!(
        build_in_process(target.path(), "run-1", "fx-rename"),
        (head.clone(), token.clone())
    );

    // A new run rebuilds it once: new build token, same SHA.
    let (next_head, next_token) = build_in_process(target.path(), "run-2", "fx-rename");
    assert_eq!(next_head, head);
    assert_ne!(next_token, token);
    assert_eq!(
        build_in_process(target.path(), "run-2", "fx-rename"),
        (next_head, next_token)
    );
}

#[test]
#[should_panic(expected = "unknown fixture 'fx-does-not-exist'")]
fn unknown_fixture_panics_with_its_name() {
    fixtures::build("fx-does-not-exist");
}

#[test]
#[should_panic(expected = "invalid fixture name")]
fn fixture_name_cannot_be_a_path() {
    fixtures::build("../fx-smoke");
}

/// Runs a bash snippet that has sourced `lib.sh`; `$1` is `dir`.
fn run_lib(snippet: &str, dir: &Path) -> Output {
    let lib = fixtures::scripts_dir().join("lib.sh");
    Command::new("bash")
        .arg("-c")
        .arg(format!(". \"$0\"\n{snippet}"))
        .arg(&lib)
        .arg(dir)
        .output()
        .unwrap()
}

fn assert_lib_fails(snippet: &str, message: &str) {
    let dir = Scratch::new("lib-fail");
    let out = run_lib(snippet, dir.path());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "expected failure of: {snippet}");
    assert!(
        stderr.contains(message),
        "stderr of `{snippet}` is {stderr:?}, expected {message:?}"
    );
}

#[test]
fn lib_sh_three_functions_make_a_commit() {
    let dir = Scratch::new("lib-ok");
    let out = run_lib(
        "fx_init \"$1\"\n\
         fx_write a/b.txt <<EOF\nline\nEOF\n\
         fx_commit 'first' '2024-05-06T07:08:09+02:00'",
        dir.path(),
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("a/b.txt")).unwrap(),
        "line\n"
    );
    assert_eq!(
        fixtures::git(dir.path(), &["log", "--format=%H|%an|%ae|%aI|%cI|%s"]),
        "a7b33fc1bd838c5cbe45e74086d2bbe6494afba0|Fixture Author|fixture@swhy.test|\
         2024-05-06T07:08:09+02:00|2024-05-06T07:08:09+02:00|first"
    );
}

#[test]
fn lib_sh_rejects_input_that_would_break_determinism() {
    // A date without an explicit offset depends on the machine's time zone.
    assert_lib_fails(
        "fx_init \"$1\"; fx_write a.txt <<<x; fx_commit msg '2024-05-06 07:08:09'",
        "date must look like",
    );
    assert_lib_fails(
        "fx_init \"$1\"; fx_write a.txt <<<x; fx_commit msg yesterday",
        "date must look like",
    );
    assert_lib_fails(
        "fx_init \"$1\"; fx_write a.txt <<<x; fx_commit msg",
        "usage: fx_commit <msg> <date>",
    );
    assert_lib_fails(
        "fx_init \"$1\"; fx_write /tmp/outside.txt <<<x",
        "path must be relative",
    );
    assert_lib_fails(
        "fx_init \"$1\"; fx_write ../outside.txt <<<x",
        "path must be relative",
    );
    assert_lib_fails("fx_write a.txt <<<x", "call fx_init first");
    assert_lib_fails(
        "mkdir -p \"$1\"; touch \"$1/file\"; fx_init \"$1\"",
        "exists and is not empty",
    );
}

#[test]
fn lib_sh_commit_without_changes_fails() {
    let dir = Scratch::new("lib-empty-commit");
    let out = run_lib(
        "fx_init \"$1\"; fx_write a.txt <<<x; fx_commit one '2024-05-06T07:08:09+00:00'; \
         fx_commit two '2024-05-07T07:08:09+00:00'",
        dir.path(),
    );
    assert!(!out.status.success());
    assert_eq!(
        fixtures::git(dir.path(), &["rev-list", "--count", "HEAD"]),
        "1"
    );
}
