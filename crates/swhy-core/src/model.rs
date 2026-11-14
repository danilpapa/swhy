//! Data model: the types of docs/TZ.md, "Термины и модель данных".
//!
//! Every type is `Serialize + Deserialize` and `Eq + Hash`. Enums are written
//! in `snake_case`, the spelling used by the `swhy/1` response (`"bugfix"`,
//! `"body_changed"`, `"guarding"`); enum variants keep serde's external
//! tagging so that the same types work with non-self-describing formats.
//!
//! Types that the specification names but does not define (`CommitId`,
//! `TaskKey`, `Confidence`, `TestFramework`, `PathStep`, `Skeleton`) are
//! described in docs/decisions.md, ADR-3.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Serialize};

/// Source language of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Swift,
    ObjC,
}

impl Lang {
    /// Prefix of a [`SymbolKey`]: `swift` or `objc`.
    pub const fn prefix(self) -> &'static str {
        match self {
            Lang::Swift => "swift",
            Lang::ObjC => "objc",
        }
    }
}

/// Key of a symbol: `<lang>:<container>.<name>(<labels>)`, for example
/// `swift:CardViewModel.reloadCards()`. Does not depend on file, module or
/// line number.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SymbolKey(pub String);

impl SymbolKey {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SymbolKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Commit SHA in hexadecimal form.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommitId(pub String);

impl CommitId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Key of a tracker task: `IOS-7391`, or `#123` for GitHub.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskKey(pub String);

impl TaskKey {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TaskKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Type,
    Method,
    Init,
    Deinit,
    Subscript,
    Property,
    Function,
    #[serde(rename = "objc_method")]
    ObjCMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// Coarse control-flow shape of a method, part of [`Skeleton`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlShape {
    pub ifs: u32,
    pub fors: u32,
    pub switches: u32,
    pub returns: u32,
}

/// Type-free skeleton of a method for cross-language matching (E9, step 4).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Skeleton {
    /// Called selectors and methods in normalised form.
    pub selectors: BTreeSet<String>,
    /// String literals without quotes and the `@` prefix.
    pub string_literals: BTreeSet<String>,
    /// Numeric constants as written.
    pub numbers: BTreeSet<String>,
    /// Bag of lower-case words from identifiers split by camelCase.
    pub words: BTreeMap<String, u32>,
    /// Mentioned types without the `NS` prefix, bridged.
    pub types: BTreeSet<String>,
    /// Number of parameters without error and completion parameters.
    pub param_count: u32,
    pub control: ControlShape,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolSnapshot {
    pub key: SymbolKey,
    pub kind: SymbolKind,
    pub lang: Lang,
    /// Path of the file in this revision.
    pub path: String,
    /// 1-based, inclusive.
    pub lines: (u32, u32),
    /// blake3 of the canonical form of the body.
    pub body_hash: [u8; 32],
    /// blake3 of the canonical signature.
    pub signature_hash: [u8; 32],
    /// For cross-language matching (E9).
    pub skeleton: Skeleton,
}

/// One step of the path from a symbol to a statement:
/// `[Body, Stmt(3), IfThen, Stmt(0)]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathStep {
    Body,
    /// Statement by 0-based index inside its block.
    Stmt(u32),
    IfThen,
}

/// Address of a statement inside a symbol. The line number is not part of it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Anchor {
    pub symbol: SymbolKey,
    pub tree_path: Vec<PathStep>,
    /// Hash of the canonical text of the statement.
    pub canon_hash: [u8; 32],
    /// As in the file, for display.
    pub raw_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Introduced,
    BodyChanged,
    SignatureChanged,
    Renamed { from: SymbolKey },
    MovedFile { from_path: String },
    MovedModule { from_module: String },
    CrossLanguage { from: SymbolKey, from_path: String },
    Split { into: Vec<SymbolKey> },
    FormatOnly,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitType {
    Bugfix,
    Feature,
    Refactoring,
    Formatting,
    Test,
    Build,
    Merge,
    Unknown,
}

/// A change of a symbol or statement in one commit.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Event {
    pub commit: CommitId,
    pub date: String,
    pub author: String,
    pub kind: EventKind,
    /// Whether the change is semantic.
    pub semantic: bool,
    /// Requested lines touched by the event, in HEAD numbering.
    pub head_lines: Vec<u32>,
    pub commit_type: CommitType,
    pub tasks: Vec<TaskKey>,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Task {
    pub key: TaskKey,
    pub title: String,
    pub kind: String,
    pub summary: String,
    pub url: String,
}

/// Chain "fix A introduced bug B, fixed by C".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegressionLink {
    /// The fix that introduced the bug.
    pub fix_a: CommitId,
    pub task_a: Option<TaskKey>,
    /// The bug.
    pub bug_b: Option<TaskKey>,
    /// The fix of the bug.
    pub fix_c: CommitId,
    pub task_c: Option<TaskKey>,
    /// Lines through which the commits are linked.
    pub shared_anchors: Vec<Anchor>,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestStrength {
    History,
    Covered,
    Guarding,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TestFramework {
    #[serde(rename = "xctest")]
    XCTest,
    #[serde(rename = "swift_testing")]
    SwiftTesting,
}

/// Link between a protective line and a test.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TestLink {
    /// `CardsTests/CardViewModelTests/test_reload()`.
    pub test_id: String,
    pub framework: TestFramework,
    pub path: String,
    pub strength: TestStrength,
    pub added_in: Option<TaskKey>,
    /// For `Covered` and `Guarding`: the HEAD on which it was verified.
    pub verified_at: Option<CommitId>,
    /// Ready-to-run command.
    pub run: String,
}

/// A line alive at HEAD that a bugfix added or changed semantically.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProtectedLine {
    pub head_line: u32,
    pub anchor: Anchor,
    pub introduced_by: CommitId,
    pub protects_against: TaskKey,
    /// This line has already been broken before.
    pub broke_before: Vec<RegressionLink>,
    pub tests: Vec<TestLink>,
    pub confidence: Confidence,
}

#[cfg(test)]
mod tests {
    use std::hash::Hash;

    use serde::de::DeserializeOwned;
    use serde_json::{Value, json};

    use super::*;

    fn key(s: &str) -> SymbolKey {
        SymbolKey(s.to_string())
    }

    fn anchor() -> Anchor {
        Anchor {
            symbol: key("swift:CardViewModel.reloadCards()"),
            tree_path: vec![
                PathStep::Body,
                PathStep::Stmt(3),
                PathStep::IfThen,
                PathStep::Stmt(0),
            ],
            canon_hash: [7; 32],
            raw_text: "guard !ids.isEmpty else { return }".to_string(),
        }
    }

    fn protected_line() -> ProtectedLine {
        ProtectedLine {
            head_line: 51,
            anchor: anchor(),
            introduced_by: CommitId("9c0d77e".to_string()),
            protects_against: TaskKey("IOS-7391".to_string()),
            broke_before: vec![RegressionLink {
                fix_a: CommitId("4f2e1aa".to_string()),
                task_a: Some(TaskKey("IOS-7325".to_string())),
                bug_b: Some(TaskKey("IOS-7391".to_string())),
                fix_c: CommitId("9c0d77e".to_string()),
                task_c: None,
                shared_anchors: vec![anchor()],
                confidence: Confidence::Medium,
            }],
            tests: vec![TestLink {
                test_id: "CardsTests/CardViewModelTests/test_reload()".to_string(),
                framework: TestFramework::XCTest,
                path: "CardsTests/CardViewModelTests.swift".to_string(),
                strength: TestStrength::Guarding,
                added_in: Some(TaskKey("IOS-7391".to_string())),
                verified_at: Some(CommitId("a1b2c3d".to_string())),
                run: "xcodebuild test -only-testing:CardsTests/CardViewModelTests/test_reload"
                    .to_string(),
            }],
            confidence: Confidence::High,
        }
    }

    fn snapshot() -> SymbolSnapshot {
        SymbolSnapshot {
            key: key("swift:CardViewModel.load(id:completion:)"),
            kind: SymbolKind::Method,
            lang: Lang::Swift,
            path: "Cards/CardViewModel.swift".to_string(),
            lines: (42, 58),
            body_hash: [1; 32],
            signature_hash: [2; 32],
            skeleton: Skeleton {
                selectors: BTreeSet::from(["fetchCards(id:)".to_string()]),
                string_literals: BTreeSet::from(["packId".to_string()]),
                numbers: BTreeSet::from(["42".to_string()]),
                words: BTreeMap::from([("cards".to_string(), 2), ("load".to_string(), 1)]),
                types: BTreeSet::from(["String".to_string()]),
                param_count: 1,
                control: ControlShape {
                    ifs: 1,
                    fors: 0,
                    switches: 0,
                    returns: 2,
                },
            },
        }
    }

    fn event(kind: EventKind) -> Event {
        Event {
            commit: CommitId("4f2e1aa".to_string()),
            date: "2025-09-18".to_string(),
            author: "Fixture Author".to_string(),
            kind,
            semantic: true,
            head_lines: vec![51, 52],
            commit_type: CommitType::Bugfix,
            tasks: vec![TaskKey("IOS-7325".to_string())],
            confidence: Confidence::High,
        }
    }

    fn round_trip<T>(value: &T) -> Value
    where
        T: Serialize + DeserializeOwned + PartialEq + fmt::Debug,
    {
        let text = serde_json::to_string(value).unwrap();
        let back: T = serde_json::from_str(&text).unwrap();
        assert_eq!(&back, value);
        // The same value always gives the same bytes.
        assert_eq!(serde_json::to_string(&back).unwrap(), text);
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn every_model_type_is_serde_eq_and_hash() {
        fn check<T: Serialize + DeserializeOwned + Eq + Hash + Clone + fmt::Debug>() {}
        check::<Lang>();
        check::<SymbolKey>();
        check::<CommitId>();
        check::<TaskKey>();
        check::<SymbolKind>();
        check::<Confidence>();
        check::<ControlShape>();
        check::<Skeleton>();
        check::<SymbolSnapshot>();
        check::<PathStep>();
        check::<Anchor>();
        check::<EventKind>();
        check::<CommitType>();
        check::<Event>();
        check::<Task>();
        check::<RegressionLink>();
        check::<TestStrength>();
        check::<TestFramework>();
        check::<TestLink>();
        check::<ProtectedLine>();
    }

    #[test]
    fn symbol_key_is_a_plain_string() {
        let k = key("objc:CardsController.-loadCardsWithId:completion:");
        assert_eq!(
            k.to_string(),
            "objc:CardsController.-loadCardsWithId:completion:"
        );
        assert_eq!(
            round_trip(&k),
            json!("objc:CardsController.-loadCardsWithId:completion:")
        );
        assert_eq!(
            round_trip(&CommitId("4f2e1aa".to_string())),
            json!("4f2e1aa")
        );
        assert_eq!(
            round_trip(&TaskKey("IOS-7391".to_string())),
            json!("IOS-7391")
        );
    }

    #[test]
    fn lang_matches_the_key_prefix() {
        assert_eq!(round_trip(&Lang::Swift), json!("swift"));
        assert_eq!(round_trip(&Lang::ObjC), json!("objc"));
        assert_eq!(Lang::Swift.prefix(), "swift");
        assert_eq!(Lang::ObjC.prefix(), "objc");
    }

    #[test]
    fn enums_use_the_spelling_of_the_response() {
        assert_eq!(round_trip(&CommitType::Bugfix), json!("bugfix"));
        assert_eq!(round_trip(&CommitType::Unknown), json!("unknown"));
        assert_eq!(round_trip(&TestStrength::Guarding), json!("guarding"));
        assert_eq!(round_trip(&TestStrength::Stale), json!("stale"));
        assert_eq!(round_trip(&Confidence::Medium), json!("medium"));
        assert_eq!(round_trip(&SymbolKind::Subscript), json!("subscript"));
        assert_eq!(round_trip(&SymbolKind::ObjCMethod), json!("objc_method"));
        assert_eq!(round_trip(&TestFramework::XCTest), json!("xctest"));
        assert_eq!(
            round_trip(&TestFramework::SwiftTesting),
            json!("swift_testing")
        );
    }

    #[test]
    fn event_kinds_round_trip() {
        assert_eq!(round_trip(&EventKind::Introduced), json!("introduced"));
        assert_eq!(round_trip(&EventKind::BodyChanged), json!("body_changed"));
        assert_eq!(
            round_trip(&EventKind::SignatureChanged),
            json!("signature_changed")
        );
        assert_eq!(round_trip(&EventKind::FormatOnly), json!("format_only"));
        assert_eq!(round_trip(&EventKind::Removed), json!("removed"));
        assert_eq!(
            round_trip(&EventKind::Renamed {
                from: key("swift:CardViewModel.refresh()")
            }),
            json!({"renamed": {"from": "swift:CardViewModel.refresh()"}})
        );
        assert_eq!(
            round_trip(&EventKind::MovedFile {
                from_path: "App/CardViewModel.swift".to_string()
            }),
            json!({"moved_file": {"from_path": "App/CardViewModel.swift"}})
        );
        assert_eq!(
            round_trip(&EventKind::MovedModule {
                from_module: "App".to_string()
            }),
            json!({"moved_module": {"from_module": "App"}})
        );
        assert_eq!(
            round_trip(&EventKind::CrossLanguage {
                from: key("objc:CardsController.-reloadCards"),
                from_path: "Legacy/CardsController.m".to_string()
            }),
            json!({"cross_language": {
                "from": "objc:CardsController.-reloadCards",
                "from_path": "Legacy/CardsController.m"
            }})
        );
        assert_eq!(
            round_trip(&EventKind::Split {
                into: vec![key("swift:A.a()"), key("swift:B.b()")]
            }),
            json!({"split": {"into": ["swift:A.a()", "swift:B.b()"]}})
        );
    }

    #[test]
    fn event_round_trips_with_fields_in_declaration_order() {
        let e = event(EventKind::BodyChanged);
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"commit":"4f2e1aa","date":"2025-09-18","author":"Fixture Author","kind":"body_changed","semantic":true,"head_lines":[51,52],"commit_type":"bugfix","tasks":["IOS-7325"],"confidence":"high"}"#
        );
        round_trip(&e);
    }

    #[test]
    fn anchor_path_matches_the_example_of_the_specification() {
        let value = round_trip(&anchor());
        assert_eq!(
            value["tree_path"],
            json!(["body", {"stmt": 3}, "if_then", {"stmt": 0}])
        );
        assert_eq!(value["symbol"], json!("swift:CardViewModel.reloadCards()"));
        assert_eq!(value["canon_hash"].as_array().unwrap().len(), 32);
    }

    #[test]
    fn nested_structures_round_trip() {
        let snapshot_json = round_trip(&snapshot());
        assert_eq!(snapshot_json["lines"], json!([42, 58]));
        assert_eq!(snapshot_json["lang"], json!("swift"));
        assert_eq!(
            snapshot_json["skeleton"]["words"],
            json!({"cards": 2, "load": 1})
        );

        let line = round_trip(&protected_line());
        assert_eq!(line["head_line"], json!(51));
        assert_eq!(line["protects_against"], json!("IOS-7391"));
        assert_eq!(line["broke_before"][0]["task_c"], Value::Null);
        assert_eq!(line["tests"][0]["strength"], json!("guarding"));

        round_trip(&Task {
            key: TaskKey("IOS-7391".to_string()),
            title: "Network crash on cache invalidation".to_string(),
            kind: "Bug".to_string(),
            summary: "Запрос уходил с id из инвалидированного кеша".to_string(),
            url: "https://jira.example.com/browse/IOS-7391".to_string(),
        });
    }
}
