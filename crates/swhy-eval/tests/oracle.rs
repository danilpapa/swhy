//! E1 oracle tooling on fixtures: candidate collection (`git log -L` ∪
//! `git log --follow` ∪ merges), the trace of a method through renames,
//! file moves and an ObjC → Swift rewrite, and the oracle file built from
//! labels. Expected steps follow the scenarios in `fixtures/fx-*.sh`.

use swhy_eval::{
    gitx::Git,
    oracle::{self, LabelBlock},
    trace::{Hint, StepKind, Tracer, sheet},
};

fn short(sha: &str) -> &str {
    &sha[..7]
}

fn kinds(git: &Git, path: &str, key: &str) -> Vec<(String, StepKind)> {
    let head = git.resolve("HEAD").unwrap();
    Tracer::new(git)
        .trace(&head, path, key)
        .unwrap()
        .steps
        .iter()
        .map(|step| (short(&step.commit).to_string(), step.kind))
        .collect()
}

fn expect(steps: &[(&str, StepKind)]) -> Vec<(String, StepKind)> {
    steps
        .iter()
        .map(|(sha, kind)| ((*sha).to_string(), *kind))
        .collect()
}

#[test]
fn trace_goes_through_a_file_move_that_git_does_not_detect() {
    let git = Git::open(&fixtures::build("fx-measure"));
    let head = git.resolve("HEAD").unwrap();
    let trace = Tracer::new(&git)
        .trace(
            &head,
            "Core/Greeting/Greeter.swift",
            "swift:Greeter.greet(name:)",
        )
        .unwrap();

    // Newest first: the move with a rewrite, the bugfix, the introduction.
    // «Reindent» and «Add copyright headers» touch the file but not greet.
    let steps: Vec<_> = trace
        .steps
        .iter()
        .map(|s| (short(&s.commit), s.kind))
        .collect();
    assert_eq!(
        steps,
        [
            ("80ad589", StepKind::MovedFromFile),
            ("272a8e4", StepKind::Changed),
            ("8498749", StepKind::Introduced),
        ]
    );
    let moved = &trace.steps[0];
    assert_eq!(moved.before.as_ref().unwrap().path, "App/Greeter.swift");
    assert_eq!(
        moved.before.as_ref().unwrap().key,
        "swift:Greeter.greet(name:)"
    );
    assert_eq!(moved.after.path, "Core/Greeting/Greeter.swift");
    assert_eq!(moved.hint, Hint::Other);
    assert!(moved.sources.contains("follow") && moved.sources.contains("log-L"));
    let fix = &trace.steps[1];
    assert_eq!(
        fix.diff,
        "+        guard !name.isEmpty else { return \"Hello\" }\n"
    );
    assert_eq!(fix.parent.as_deref().map(short), Some("15f8663"));
    assert_eq!(trace.steps[2].before, None);

    // Candidates: Core/Greeting/Greeter.swift has 1 commit, App/Greeter.swift
    // 4 commits before the move (add, fix, reindent, headers); log -L gives
    // 1 commit for the new file and 2 (add, fix) for the old one.
    assert_eq!(trace.candidates.follow, 5);
    assert_eq!(trace.candidates.log_l, 3);
    assert_eq!(trace.candidates.merges, 0);
    assert_eq!(trace.candidates.union, 5);
    assert_eq!(trace.candidates.tool, 0);
}

#[test]
fn whitespace_only_change_gets_a_hint() {
    let git = Git::open(&fixtures::build("fx-measure"));
    let head = git.resolve("HEAD").unwrap();
    let trace = Tracer::new(&git)
        .trace(&head, "App/Config.swift", "swift:Config.isValid()")
        .unwrap();
    let steps: Vec<_> = trace
        .steps
        .iter()
        .map(|s| (short(&s.commit), s.kind, s.hint))
        .collect();
    assert_eq!(
        steps,
        [
            ("06d8dc9", StepKind::Changed, Hint::WhitespaceOnly),
            ("8498749", StepKind::Introduced, Hint::Other),
        ]
    );
}

#[test]
fn trace_continues_into_the_objc_method() {
    let git = Git::open(&fixtures::build("fx-measure"));
    assert_eq!(
        kinds(&git, "Core/Math.swift", "swift:Math.addValue(_:to:)"),
        expect(&[
            ("0c8ed60", StepKind::CrossLanguage),
            ("15f8663", StepKind::Introduced)
        ])
    );
    let head = git.resolve("HEAD").unwrap();
    let trace = Tracer::new(&git)
        .trace(&head, "Core/Math.swift", "swift:Math.addValue(_:to:)")
        .unwrap();
    let cross = &trace.steps[0];
    let before = cross.before.as_ref().unwrap();
    assert_eq!(before.path, "Legacy/OldMath.m");
    assert_eq!(before.key, "objc:OldMath.-addValue:to:");
    assert_eq!(trace.steps[1].after.key, "objc:OldMath.-addValue:to:");

    // A cut stops the trace: the Swift method counts as introduced there.
    let mut cut = Tracer::new(&git);
    cut.cuts.insert(git.resolve("0c8ed60").unwrap());
    let trace = cut
        .trace(&head, "Core/Math.swift", "swift:Math.addValue(_:to:)")
        .unwrap();
    assert_eq!(trace.steps.len(), 1);
    assert_eq!(trace.steps[0].kind, StepKind::Introduced);

    // A method without a namesake in the deleted files is introduced.
    assert_eq!(
        kinds(
            &git,
            "Core/Greeting/Greeter.swift",
            "swift:Greeter.welcome(guests:)"
        ),
        expect(&[("80ad589", StepKind::Introduced)])
    );
}

#[test]
fn trace_of_fx_rename_goes_through_extension_move_and_rename() {
    let git = Git::open(&fixtures::build("fx-rename"));
    // 917c1cb body change; 9462e00 move into the extension file; 24009f1
    // rename refresh -> reloadCards; b63dc21 introduction. The merge commit
    // eac1aee equals its second parent and is not a step.
    assert_eq!(
        kinds(
            &git,
            "Cards/CardViewModel+Reload.swift",
            "swift:CardViewModel.reloadCards()"
        ),
        expect(&[
            ("917c1cb", StepKind::Changed),
            ("9462e00", StepKind::MovedFromFile),
            ("24009f1", StepKind::Renamed),
            ("b63dc21", StepKind::Introduced),
        ])
    );
    let head = git.resolve("HEAD").unwrap();
    let trace = Tracer::new(&git)
        .trace(
            &head,
            "Cards/CardViewModel+Reload.swift",
            "swift:CardViewModel.reloadCards()",
        )
        .unwrap();
    assert_eq!(
        trace.steps[2].before.as_ref().unwrap().key,
        "swift:CardViewModel.refresh()"
    );
    assert_eq!(trace.steps[3].after.key, "swift:CardViewModel.refresh()");
    assert_eq!(trace.candidates.merges, 1);

    // Overloads are told apart by labels; the label change is a rename step.
    assert_eq!(
        kinds(
            &git,
            "Cards/CardViewModel.swift",
            "swift:CardViewModel.load(for:)"
        ),
        expect(&[
            ("a407d87", StepKind::Renamed),
            ("b63dc21", StepKind::Introduced)
        ])
    );
    assert_eq!(
        kinds(
            &git,
            "Cards/CardViewModel.swift",
            "swift:CardViewModel.load(id:completion:)"
        ),
        expect(&[
            ("07c321b", StepKind::Changed),
            ("b63dc21", StepKind::Introduced)
        ])
    );
}

fn block(labels: &[(&str, &str)], cuts: &[&str]) -> LabelBlock {
    LabelBlock {
        id: "fx-measure-01".to_string(),
        split: "train".to_string(),
        path: "Core/Math.swift".to_string(),
        key: "swift:Math.addValue(_:to:)".to_string(),
        labels: labels
            .iter()
            .map(|(c, l)| ((*c).to_string(), (*l).to_string(), None))
            .collect(),
        cuts: cuts.iter().map(ToString::to_string).collect(),
    }
}

#[test]
fn oracle_file_is_built_from_trace_and_labels() {
    let git = Git::open(&fixtures::build("fx-measure"));
    let head = git.resolve("HEAD").unwrap();
    let labels = block(&[("0c8ed60", "Semantic"), ("15f8663", "Introduced")], &[]);
    let trace = oracle::trace_block(&git, &head, &labels).unwrap();
    let method = oracle::build_method("fx-measure", &head, "tester", &labels, &trace).unwrap();

    assert_eq!(method.schema, "swhy-oracle-method/1");
    assert_eq!(method.introduced, git.resolve("15f8663").unwrap());
    assert!(method.objc_origin);
    // Legacy/ -> Core/: the top-level directory changes.
    assert!(method.module_move);
    assert_eq!(method.lines, (5, 7));
    assert_eq!(method.changing_commits(), 1);
    // Oldest first.
    let history: Vec<_> = method
        .history
        .iter()
        .map(|c| {
            (
                short(&c.commit),
                c.class.as_str(),
                c.move_kind.as_deref(),
                c.key.as_str(),
            )
        })
        .collect();
    assert_eq!(
        history,
        [
            ("15f8663", "Introduced", None, "objc:OldMath.-addValue:to:"),
            (
                "0c8ed60",
                "Semantic",
                Some("cross_language"),
                "swift:Math.addValue(_:to:)"
            ),
        ]
    );
    assert_eq!(
        method.history[1].from.as_ref().unwrap().path,
        "Legacy/OldMath.m"
    );
    assert_eq!(method.history[1].sources, ["follow", "log-L"]);

    // Byte-identical on a second build; round-trips through JSON.
    let again = oracle::build_method("fx-measure", &head, "tester", &labels, &trace).unwrap();
    assert_eq!(oracle::to_json(&method), oracle::to_json(&again));
    let parsed: oracle::OracleMethod = serde_json::from_str(&oracle::to_json(&method)).unwrap();
    assert_eq!(parsed, method);

    // The sheet shows both versions of a cross-language step.
    let text = sheet("fx-measure-01", &trace, 1000);
    assert!(text.contains("CrossLanguage"));
    assert!(text.contains("from objc:OldMath.-addValue:to: @ Legacy/OldMath.m"));
    assert!(text.contains("- (NSInteger)addValue:(NSInteger)a to:(NSInteger)b {"));
    assert!(text.contains("    return a + b;"));
    let brief = sheet("fx-measure-01", &trace, 1);
    assert!(!brief.contains("    return a + b;"));
    assert!(brief.contains("… 2 more lines"));
}

#[test]
fn every_step_needs_exactly_one_label() {
    let git = Git::open(&fixtures::build("fx-measure"));
    let head = git.resolve("HEAD").unwrap();
    let build = |labels: &LabelBlock| {
        let trace = oracle::trace_block(&git, &head, labels)?;
        oracle::build_method("fx-measure", &head, "tester", labels, &trace)
    };

    let missing = build(&block(&[("15f8663", "Introduced")], &[])).unwrap_err();
    assert!(
        missing.contains("0c8ed60") && missing.contains("no label"),
        "{missing}"
    );
    let unknown = build(&block(
        &[
            ("0c8ed60", "Semantic"),
            ("15f8663", "Introduced"),
            ("7a1d762", "Semantic"),
        ],
        &[],
    ))
    .unwrap_err();
    assert!(unknown.contains("matches no step"), "{unknown}");
    let wrong_end = build(&block(
        &[("0c8ed60", "Introduced"), ("15f8663", "Introduced")],
        &[],
    ))
    .unwrap_err();
    assert!(wrong_end.contains("Introduced must match"), "{wrong_end}");

    // With a cut the Swift method is introduced by the conversion commit.
    let cut = build(&block(&[("0c8ed60", "Introduced")], &["0c8ed60"])).unwrap();
    assert!(!cut.objc_origin && !cut.module_move);
    assert_eq!(cut.history.len(), 1);

    // A reviewed commit judged not changing is kept apart from the history.
    let labels = LabelBlock {
        path: "App/Config.swift".to_string(),
        key: "swift:Config.isValid()".to_string(),
        ..block(
            &[("06d8dc9", "NotChanging"), ("8498749", "Introduced")],
            &[],
        )
    };
    let method = build(&labels).unwrap();
    assert_eq!(method.history.len(), 1);
    assert_eq!(method.not_changing, [git.resolve("06d8dc9").unwrap()]);
}

#[test]
fn check_reports_violations_and_kappa() {
    let git = Git::open(&fixtures::build("fx-measure"));
    let head = git.resolve("HEAD").unwrap();
    let labels = LabelBlock {
        path: "App/Config.swift".to_string(),
        key: "swift:Config.isValid()".to_string(),
        ..block(&[("06d8dc9", "FormatOnly"), ("8498749", "Introduced")], &[])
    };
    let trace = oracle::trace_block(&git, &head, &labels).unwrap();
    let first = oracle::build_method("fx-measure", &head, "first", &labels, &trace).unwrap();
    let mut second = first.clone();
    second.annotator = "second".to_string();

    // One method, no second annotator: every requirement of the spec fails.
    let alone = oracle::check(std::slice::from_ref(&first), &[]).unwrap();
    assert_eq!(alone.methods, 1);
    assert_eq!((alone.train, alone.test), (1, 0));
    assert_eq!(alone.min_changing_commits, 1);
    assert_eq!(alone.methods_below_min, ["fx-measure-01"]);
    assert_eq!(alone.second_annotator.methods_required, 1);
    assert_eq!(alone.second_annotator.class, None);
    assert_eq!(alone.violations.len(), 7);
    assert!(
        alone
            .violations
            .contains(&"Cohen's kappa is not computed".to_string())
    );
    assert_eq!(alone.commits_by_class["FormatOnly"], 1);

    // Full agreement on two items of two classes: kappa 1.
    let agreed =
        oracle::check(std::slice::from_ref(&first), std::slice::from_ref(&second)).unwrap();
    let class = agreed.second_annotator.class.unwrap();
    assert_eq!((class.items, class.agreed, class.kappa), (2, 2, Some(1.0)));
    assert_eq!(agreed.second_annotator.annotators, ["second"]);

    // The second annotator calls the reindent not changing: one disagreement.
    second.not_changing.push(second.history.remove(1).commit);
    let disagreed =
        oracle::check(std::slice::from_ref(&first), std::slice::from_ref(&second)).unwrap();
    let class = disagreed.second_annotator.class.unwrap();
    assert_eq!((class.items, class.agreed), (2, 1));
    assert_eq!(disagreed.second_annotator.changed.unwrap().agreed, 1);

    // The same annotator twice is not a second annotator.
    assert!(oracle::check(std::slice::from_ref(&first), std::slice::from_ref(&first)).is_err());
}
