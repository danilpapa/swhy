//! The committed oracle `eval/oracle/methods/*.json` against the numbers of
//! docs/TZ.md, E1, «Эталон истории методов». Needs no clone of the dataset.

use std::collections::BTreeSet;

use swhy_eval::{
    oracle::{self, OracleMethod},
    repos, workspace_root,
};

fn methods() -> Vec<OracleMethod> {
    oracle::load_dir(&workspace_root().join("eval/oracle/methods")).unwrap()
}

#[test]
fn fifty_methods_of_signal_ios_at_the_pinned_revision() {
    let methods = methods();
    let repo = repos::Repo::find("Signal-iOS").unwrap();
    assert_eq!(methods.len(), 50);
    for (index, method) in methods.iter().enumerate() {
        // «Формат: `eval/oracle/methods/<repo>-<n>.json`.»
        assert_eq!(method.id, format!("signal-ios-{:02}", index + 1));
        assert_eq!(method.schema, "swhy-oracle-method/1");
        assert_eq!(method.repo, "Signal-iOS");
        assert_eq!(method.rev, repo.sha);
        assert!(method.method.key.starts_with("swift:"), "{}", method.id);
        assert!(method.method.path.ends_with(".swift"), "{}", method.id);
        assert!(!method.annotator.is_empty());
    }
    let distinct: BTreeSet<(&str, &str)> = methods
        .iter()
        .map(|m| (m.method.path.as_str(), m.method.key.as_str()))
        .collect();
    assert_eq!(distinct.len(), 50);
}

#[test]
fn split_is_25_for_tuning_and_25_for_verification() {
    let methods = methods();
    assert_eq!(methods.iter().filter(|m| m.split == "train").count(), 25);
    assert_eq!(methods.iter().filter(|m| m.split == "test").count(), 25);
}

#[test]
fn every_method_has_at_least_3_changing_commits_and_one_introduction() {
    for method in methods() {
        assert!(method.changing_commits() >= 3, "{}", method.id);
        let first = &method.history[0];
        assert_eq!(first.class, "Introduced", "{}", method.id);
        assert_eq!(first.commit, method.introduced, "{}", method.id);
        assert_eq!(
            method
                .history
                .iter()
                .filter(|c| c.class == "Introduced")
                .count(),
            1
        );
        for commit in &method.history {
            assert!(
                oracle::CLASSES.contains(&commit.class.as_str()),
                "{}",
                commit.class
            );
            assert_eq!(commit.commit.len(), 40);
            assert!(!commit.sources.is_empty());
            if let Some(kind) = &commit.move_kind {
                assert!(["file", "module", "cross_language"].contains(&kind.as_str()));
                assert!(commit.from.is_some(), "{} {}", method.id, commit.commit);
            }
        }
        // Oldest first.
        let dates: Vec<&str> = method.history.iter().map(|c| c.date.as_str()).collect();
        assert_eq!(dates[0], *dates.iter().min().unwrap(), "{}", method.id);
        // The last commit of the history holds the method of the pinned revision.
        let last = method.history.last().unwrap();
        assert_eq!(
            (&last.path, &last.key),
            (&method.method.path, &method.method.key)
        );
        // Every candidate source of the stage is counted; the tool has none yet.
        assert!(method.candidates.follow > 0 && method.candidates.log_l > 0);
        assert!(method.candidates.union >= method.history.len());
        assert_eq!(method.candidates.tool, 0);
    }
}

#[test]
fn at_least_10_methods_with_a_module_move_and_10_with_objc_origin() {
    let methods = methods();
    let module_move = methods.iter().filter(|m| m.module_move).count();
    let objc_origin = methods.iter().filter(|m| m.objc_origin).count();
    assert!(module_move >= 10, "{module_move}");
    assert!(objc_origin >= 10, "{objc_origin}");
    for method in &methods {
        // The flags follow from the history.
        let crosses = method.history.iter().any(|c| {
            c.from
                .as_ref()
                .is_some_and(|from| from.path.split('/').next() != c.path.split('/').next())
        });
        assert_eq!(method.module_move, crosses, "{}", method.id);
        let objc = method.history.iter().any(|c| c.key.starts_with("objc:"));
        assert_eq!(method.objc_origin, objc, "{}", method.id);
        if method.objc_origin {
            assert!(method.history[0].key.starts_with("objc:"), "{}", method.id);
        }
    }
}

#[test]
fn check_passes_everything_but_the_second_annotator() {
    let metrics = oracle::check(&methods(), &[]).unwrap();
    assert_eq!(metrics.methods, 50);
    assert_eq!((metrics.train, metrics.test), (25, 25));
    assert!(metrics.methods_below_min.is_empty());
    assert_eq!(metrics.second_annotator.methods_required, 15);
    // Whatever is left in `violations` concerns the second annotator only.
    for violation in &metrics.violations {
        assert!(
            violation.starts_with("second annotator") || violation.contains("kappa"),
            "{violation}"
        );
    }
}
