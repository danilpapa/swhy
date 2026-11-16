# ТЗ для агента: why — история Swift-кода

Nov 9, 2026 · @danilpapa

## Как работать по ТЗ

ТЗ — единственный источник требований. Агент идёт по этапам E0 → E17 по порядку и не начинает следующий этап, пока не выполнены критерии приёмки текущего. Исключение — этапы, явно помеченные как параллельные.

**Имя инструмента.** Имя `why` занято: есть Rust CLI `why` (quangdang46) с командой `why mcp`. Поэтому в коде везде `swhy`: бинарь `swhy`, крейты `swhy-*`, конфиг `.swhy.toml`, переменная `SWHY_TRACKER_TOKEN`, схема ответа `swhy/1`. В тексте ТЗ для краткости написано `why`, `why-core`, `.why.toml` — при реализации читать как `swhy`, `swhy-core`, `.swhy.toml`.

**Цикл одного этапа:**

1. Прочитать раздел этапа целиком, включая «Входы» и «Критерии приёмки».
2. Сверить версии зависимостей в `Cargo.lock` с docs.rs. API библиотек не выдумывать: если метод не найден в документации нужной версии — найти аналог или записать проблему в `docs/decisions.md`.
3. Сначала написать тесты на критерии приёмки (fixture-репозиторий + ожидаемый JSON), потом код.
4. Прогнать `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`.
5. Обновить `docs/progress.md`: этап, что сделано, цифры метрик, известные ограничения.
6. Коммит с префиксом `E<n>:`, например `E4: symbol tracking across renames`.

**Постоянные правила:**

- Никакой фоновой индексации. Всё считается по запросу, результат кешируется. Прогрев кеша — только явной командой `why cache warm`.
- Код пользователя никуда не отправляется. Запросы к Jira, GitHub и LLM — только если они включены в `.why.toml` или флагом.
- Формат ответа (раздел «Интерфейсы») меняется только вместе с JSON-схемой, версией `schema` и snapshot-тестами.
- Любое решение, которого нет в ТЗ, записывается в `docs/decisions.md` в формате ADR: контекст, решение, отвергнутые варианты.
- Каждая метрика считается скриптом из `eval/`, запускается одной командой и пишет результат в `eval/results/<этап>-<дата>.json`. Цифры в тексте диплома берутся только оттуда.
- Детерминизм: одинаковый вход даёт побайтово одинаковый JSON (сортировка, без времени генерации в теле ответа). Это нужно для кэша промптов агента и snapshot-тестов.
- Ошибка разбора одного файла или одной ревизии не роняет запрос: событие помечается `"confidence": "low"`, причина пишется в `warnings`.

**Окружение:** macOS 15 или новее, Xcode 26, Rust stable (версия закреплена в `rust-toolchain.toml`), git 2.45+. Linux поддерживается для всего, кроме прогона iOS-тестов.

**Если не успеваешь:** порядок урезания — в разделе «Риски и запасные варианты». Ядро новизны (N1–N4 из следующего раздела) не урезается.

## Цель, границы и новизна

**Цель:** инструмент `why`, который по запросу «файл + диапазон строк» возвращает компактную историю этих строк. История переживает рефакторинги, переезды и переписывание с ObjC на Swift. В ответе есть защитные строки, цепочки регрессий и тесты, которые эти строки защищают. Инструмент снижает стоимость исправления бага ИИ-агентом без регрессий.

**Тема диплома:** «Метод и инструмент рефакторинг-устойчивого анализа истории Swift-кода: защитные строки, цепочки регрессий и связанные регрессионные тесты для ИИ-агентов и разработчиков в крупных iOS-приложениях».

**Входит в работу:**

- языки Swift и Objective-C (`.swift`, `.m`, `.h`, `.mm` — только как ObjC);
- история диапазона строк, символа и файла;
- переезды между файлами и модулями, разбиение файла, переписывание ObjC → Swift на уровне методов;
- каноническая форма Swift и классификация изменений;
- тип коммита, задачи из Jira и GitHub Issues;
- защитные строки, цепочки «фикс A внёс баг B, исправлено в C»;
- связь строк с тестами трёх уровней: `history`, `covered`, `guarding`;
- проверка перед коммитом (`why guard`);
- CLI, MCP-сервер, приложение на Tauri;
- датасет, бенчмарк, эксперименты.

**Не входит:** фоновая индексация; сборка старых ревизий ради типов; языки кроме Swift и ObjC; автоматическое исправление кода; YouTrack (только интерфейс-заглушка); облачный сервис.

**Новизна, которую нельзя потерять.** Каждый пункт должен закончиться реализацией и измеренной метрикой.

| № | Что новое | Чем подтверждаем | Этапы |
| --- | --- | --- | --- |
| N1 | Рефакторинг-устойчивая история **диапазона строк** Swift и ObjC, включая переезды между модулями и межъязыковое переписывание ObjC → Swift | Точность и полнота истории на размеченном эталоне против `git log -L` и `git blame -M -C` | E3–E7, E9 |
| N2 | Защитные строки и **прямые** цепочки регрессий «фикс A → баг B → фикс C», переживающие рефакторинги | Точность и полнота на ручной разметке; ретроспектива: предупредил бы `why guard` в момент фикса A | E10, E11, E13 |
| N3 | Связь защитной строки с регрессионными тестами; уровень `guarding` проверяет тест **откатом конкретного исторического фикса** | Доля защитных строк с тестом; доля тестов `history`, подтверждённых как `guarding` | E12 |
| N4 | Компактное представление истории для ИИ-агента и экспериментальное обоснование его состава | Стоимость решённой задачи без регрессий в трёх режимах агента и трёх вариантах формата | E14, E16, E17 |
| N5 | Каноническая форма Swift для отделения смысловых изменений от технических | Доля технических коммитов, скрытых без потери смысловых; точность на разметке | E6 |
| N6 | Эмпирические данные по iOS: как часто рвётся история и какая доля фиксов вносит новые баги | Цифры по 6–8 открытым iOS-репозиториям | E1, E11 |

Фундамент (отслеживание методов, SZZ, GumTree) берётся из известных работ и адаптируется к Swift. В тексте диплома это прямо названо заимствованием, новизна — только N1–N6.

**Ближайшие аналоги — назвать в обзоре и сравнении:**

| Инструмент или работа | Что умеет | Чего нет |
| --- | --- | --- |
| `why` (Rust CLI, quangdang46) | tree-sitter + blame, история символа с учётом переименований, оценка риска HIGH/MEDIUM/LOW через LLM, сигналы hotfix и ссылки на задачи, MCP-сервер | Swift и ObjC, защитные строки, цепочки, тесты, Jira |
| Codebase Archaeology MCP | `why_this_line`, история строки через переезды и переименования, обсуждения PR, пропуск форматирования | обрывается на разбиении файла; нет классификации багфиксов, задач, тестов, Swift |
| GitKraken MCP | `git_blame`, задачи, комментарии PR | нет «почему», нет учёта рефакторингов |
| `find-regression-risk` в dotnet/maui | помечает PR, удаляющий строку недавнего багфикса | точное сравнение строк, без blame, рефакторингов, цепочек |
| HAFix, HAFixAgent | история blame в промпте агента починки багов: +4,4% (Defects4J) и +38,6% (BugsInPy) | не инструмент, нет защитных строк, Java и Python |
| CodeShovel, CodeTracker, HistoryFinder, RefactoringMiner | история методов и блоков сквозь рефакторинги | Swift и ObjC не поддерживают |

**Формулировки, которых избегать:** «первый инструмент, объясняющий, зачем существует строка», «первый MCP для истории git», «первый агент с историей», «первый, кто предупреждает о рискованных строках». Безопасно: «для Swift и ObjC», «защитные строки и цепочки регрессий с учётом рефакторингов», «связь строк с тестами через откат исторического фикса», «измеренный эффект на стоимость и регрессии агента» — с оговоркой «в рассмотренных работах не найдено».

## Термины и модель данных

Все типы живут в крейте `why-core`, модуль `model`. Все они `Serialize + Deserialize` (serde) и `Eq + Hash`, где это возможно.

**Термины:**

| Термин | Определение |
| --- | --- |
| Символ | Именованная единица кода: тип, метод, init, subscript, свойство, ObjC-метод. Имеет ключ `SymbolKey` |
| Ключ символа | Строка, которая однозначно называет символ в одной ревизии и не зависит от номера строки и файла |
| Якорь | Адрес оператора внутри символа: ключ символа + путь в дереве + хеш канонического текста. Номер строки — только для показа |
| Событие | Изменение символа или оператора в одном коммите: появление, правка тела, переименование, переезд, межъязыковой переезд, удаление |
| Смысловое изменение | Изменение, после которого канонические формы до и после различаются |
| Технический коммит | Коммит, все изменения которого в запрошенных строках несмысловые: форматирование, переезд, переименование без правки тела |
| Защитная строка | Живая сейчас строка, добавленная или изменённая коммитом-багфиксом, причём это изменение смысловое |
| Цепочка регрессий | Фикс A внёс строки, которые SZZ указал как причину бага B; B исправлен коммитом C |
| Связь с тестом | Пара «защитная строка — тест» с уровнем силы `history`, `covered`, `guarding` или `stale` |

**Формат ключа символа.** Строка `<lang>:<контейнер>.<имя>(<метки>)`, контейнер — цепочка вложенных типов. Модуль и файл в ключ не входят: при переезде между модулями ключ не меняется.

| Код | Ключ |
| --- | --- |
| `func reloadCards()` в `class CardViewModel` | `swift:CardViewModel.reloadCards()` |
| `func load(id: String, completion: …)` | `swift:CardViewModel.load(id:completion:)` |
| `func load(_ id: String)` | `swift:CardViewModel.load(_:)` |
| `init(api: API)` | `swift:CardViewModel.init(api:)` |
| `subscript(index: Int)` | `swift:CardViewModel.subscript(index:)` |
| `var isEmpty: Bool { … }` | `swift:CardViewModel.isEmpty` |
| метод в `extension CardViewModel` в другом файле | `swift:CardViewModel.reloadCards()` — тот же контейнер |
| вложенный тип `CardViewModel.State.reset()` | `swift:CardViewModel.State.reset()` |
| глобальная функция | `swift:<global>.formatAmount(_:)` |
| `- (void)loadCardsWithId:(NSString *)id completion:…` | `objc:CardsController.-loadCardsWithId:completion:` |
| `+ (instancetype)sharedInstance` | `objc:CardsController.+sharedInstance` |
| метод в категории `CardsController (Network)` | `objc:CardsController.-…` — категория в ключ не входит |

Однофамильцы в одной ревизии (две глобальные функции с одним ключом в разных модулях, `#if` с двумя реализациями) различаются суффиксом `#2`, `#3` в порядке «путь файла, затем позиция». Суффикс считается в каждой ревизии заново.

**Структуры (Rust):**

```rust
pub enum Lang { Swift, ObjC }

pub struct SymbolKey(pub String);           // "swift:CardViewModel.reloadCards()"

pub enum SymbolKind { Type, Method, Init, Deinit, Subscript, Property, Function, ObjCMethod }

pub struct SymbolSnapshot {
    pub key: SymbolKey,
    pub kind: SymbolKind,
    pub lang: Lang,
    pub path: String,                       // путь файла в этой ревизии
    pub lines: (u32, u32),                  // 1-based, включительно
    pub body_hash: [u8; 32],                // blake3 канонической формы тела
    pub signature_hash: [u8; 32],           // blake3 канонической сигнатуры
    pub skeleton: Skeleton,                 // для межъязыкового сопоставления (E9)
}

pub struct Anchor {
    pub symbol: SymbolKey,
    pub tree_path: Vec<PathStep>,           // [Body, Stmt(3), IfThen, Stmt(0)]
    pub canon_hash: [u8; 32],               // хеш канонического текста оператора
    pub raw_text: String,                   // как в файле, для показа
}

pub enum EventKind {
    Introduced, BodyChanged, SignatureChanged, Renamed { from: SymbolKey },
    MovedFile { from_path: String }, MovedModule { from_module: String },
    CrossLanguage { from: SymbolKey, from_path: String },
    Split { into: Vec<SymbolKey> }, FormatOnly, Removed,
}

pub struct Event {
    pub commit: CommitId, pub date: String, pub author: String,
    pub kind: EventKind,
    pub semantic: bool,                     // смысловое ли изменение
    pub head_lines: Vec<u32>,               // какие из запрошенных строк задеты, в нумерации HEAD
    pub commit_type: CommitType,
    pub tasks: Vec<TaskKey>,
    pub confidence: Confidence,             // High | Medium | Low
}

pub enum CommitType { Bugfix, Feature, Refactoring, Formatting, Test, Build, Merge, Unknown }

pub struct Task { pub key: TaskKey, pub title: String, pub kind: String, pub summary: String, pub url: String }

pub struct ProtectedLine {
    pub head_line: u32, pub anchor: Anchor,
    pub introduced_by: CommitId, pub protects_against: TaskKey,
    pub broke_before: Vec<RegressionLink>,  // эта строка уже ломалась
    pub tests: Vec<TestLink>,
    pub confidence: Confidence,
}

pub struct RegressionLink {
    pub fix_a: CommitId, pub task_a: Option<TaskKey>,   // фикс, который внёс баг
    pub bug_b: Option<TaskKey>,                         // баг
    pub fix_c: CommitId, pub task_c: Option<TaskKey>,   // фикс бага
    pub shared_anchors: Vec<Anchor>,                    // строки, через которые связаны
    pub confidence: Confidence,
}

pub enum TestStrength { History, Covered, Guarding, Stale }

pub struct TestLink {
    pub test_id: String,                    // "CardsTests/CardViewModelTests/test_reload()"
    pub framework: TestFramework,           // XCTest | SwiftTesting
    pub path: String,
    pub strength: TestStrength,
    pub added_in: Option<TaskKey>,
    pub verified_at: Option<CommitId>,      // для Covered/Guarding — на каком HEAD проверено
    pub run: String,                        // готовая команда запуска
}
```

## Архитектура и стек

Одно ядро на Rust (`swhy-core`) и три тонких входа: CLI, MCP-сервер и приложение на Tauri. Версии проверены на crates.io и npm 9 ноября 2026, ключевые API проверены сборкой тестовых программ.

**Структура репозитория:**

```
swhy/
├── Cargo.toml              # workspace, edition = "2024", rust-version = "1.89"
├── rust-toolchain.toml
├── crates/
│   ├── swhy-core/src/
│   │   ├── model.rs        # типы из раздела «Термины и модель данных»
│   │   ├── parse/          # swift.rs, objc.rs, objc_prepass.rs, symbols.rs, canon.rs, skeleton.rs
│   │   ├── git/            # repo.rs, walk.rs, diff.rs, blame.rs
│   │   ├── track/          # symbol.rs (E4), moves.rs (E5), lines.rs (E7), lhdiff.rs, crosslang.rs (E9)
│   │   ├── classify.rs     # E6 классы изменений, E8 тип коммита
│   │   ├── tracker/        # mod.rs, jira.rs, github.rs
│   │   ├── protect.rs      # E10
│   │   ├── chains.rs       # E11
│   │   ├── tests/          # discover.rs, history.rs, coverage.rs, guarding.rs (E12)
│   │   ├── guard.rs        # E13
│   │   ├── response.rs     # сборка и бюджет ответа swhy/1
│   │   ├── cache.rs        # SQLite
│   │   └── query.rs        # pub fn query(Request) -> Response
│   ├── swhy-cli/           # clap
│   ├── swhy-mcp/           # rmcp, stdio
│   └── swhy-eval/          # инструменты эталонов и метрик
├── app/                    # Tauri: src-tauri/ (зависит от swhy-core) + ui/ (TypeScript, Vite, CodeMirror 6)
├── fixtures/               # скрипты fx-*.sh, которые создают маленькие git-репозитории
├── bench/tasks/            # задачи E16
├── eval/                   # скрипты и результаты метрик
└── docs/                   # progress.md, decisions.md, oracle-guide.md
```

**Зависимости:**

| Крейт / пакет | Версия | Фичи и заметки |
| --- | --- | --- |
| `tree-sitter` | 0.27.2 | единая версия на весь workspace |
| `tree-sitter-swift` | 0.7.4 | грамматика alex-pinkus, ABI 15 |
| `tree-sitter-objc` | 3.0.2 | ABI 14, последний релиз декабрь 2024 |
| `gix` | 0.90.0 | `default-features = false`, `features = ["blame", "blob-diff", "revision", "max-performance", "sha1"]` |
| `git2` | 0.21.0 | опционально, фича `git2-baseline`: blame с поиском копий для базовой линии |
| `imara-diff` | 0.2.0 | алгоритм Histogram |
| `strsim` | последняя | Jaro-Winkler, Levenshtein |
| `rusqlite` | 0.41.1 | `bundled`; из-за него MSRV 1.89 |
| `serde`, `serde_json` | 1.0.229, 1.0.154 |  |
| `blake3` | 1.8.7 | хеши тел и якорей |
| `rayon` | 1.12.0 | разбор файлов коммита параллельно |
| `clap` | 4.6.7 | `derive` |
| `rmcp` | 3.9.0 | `server`, `macros`, `transport-io`, `schemars`; вместе с `schemars = "1"`, `tokio` |
| `reqwest` | 0.13.5 | `json`, `rustls-tls` |
| `tracing`, `tracing-subscriber` | последние | логи только в stderr |
| `tauri` | 2.12.5 | `@tauri-apps/api` 2.12.4, `@tauri-apps/cli` 2.12.4 |
| `@codemirror/view` | 6.43.15 | плюс `@codemirror/state`, `@codemirror/legacy-modes` (Swift, ObjC) |
| `insta`, `wiremock` | последние | dev: snapshot-тесты, мок HTTP |

**Подводные камни API — проверено, обязательно учесть:**

- **tree-sitter 0.27.** Грамматика подключается как `let lang: Language = tree_sitter_swift::LANGUAGE.into();`. Функции `language()` больше нет — README tree-sitter-swift устарел. Результаты запроса — `while let Some(m) = it.next()` (трейт `StreamingIterator`), захваты — метод `m.captures()`, не поле.
- **tree-sitter-swift: виды узлов.** `class_declaration` — это и class, и struct, и enum, и extension, и actor; различать по полю `declaration_kind`. У extension имя — узел `user_type`. Протокол — `protocol_declaration`, его члены — `protocol_function_declaration`, `protocol_property_declaration`. Остальное: `function_declaration`, `init_declaration`, `deinit_declaration`, `subscript_declaration`, `property_declaration`, замыкание — `lambda_literal`.
- **tree-sitter-swift: поле `name` ловушка.** Поле `name` навешано и на тип результата, и на тип параметра. Имя брать только через `child_by_field_name("name")` (вернёт первое — само имя), тип — через `return_type` и `type`. У `subscript_declaration` имени нет вовсе: ключ строится как `subscript(<метки>:)`.
- **tree-sitter-swift: параметры** — прямые дети объявления, без обёртки. Внешняя метка — поле `external_name` (для `_` это `simple_identifier` с текстом `_`). Нет `external_name` — метка равна внутреннему имени. Проверенный код построения сигнатуры:

```rust
let name = n.child_by_field_name("name")?.utf8_text(src)?;      // "load", для init — "init"
let mut c = n.walk();
let labels: Vec<String> = n.named_children(&mut c)
    .filter(|x| x.kind() == "parameter")
    .map(|p| match p.child_by_field_name("external_name") {
        Some(e) => e.utf8_text(src).unwrap().to_string(),       // "from" или "_"
        None => p.child_by_field_name("name").unwrap().utf8_text(src).unwrap().to_string(),
    })
    .collect();
let sig = format!("{name}({})", labels.iter().map(|l| format!("{l}:")).collect::<String>());
```

- **tree-sitter-swift: `#if`.** Директивы — плоские соседние узлы `directive`, не контейнеры. Символы внутри `#if DEBUG … #endif` находить, проходя соседей между директивами. Три известных сбоя разбора: `#if` между `case` внутри `switch`, `#if` вокруг одного атрибута, `do`-выражение (`let x = do { … } catch { … }`). Для символа с `node.has_error()` — откат на построчное отслеживание (LHDiff), в ответе `confidence: low`.
- **tree-sitter-objc: предобработка обязательна.** Без неё `NS_ASSUME_NONNULL_BEGIN/END`, `NS_DESIGNATED_INITIALIZER`, `NS_ENUM`, `NS_OPTIONS` ломают разбор всего `@interface`. Перед разбором заменить пробелами той же длины `NS_ASSUME_NONNULL_BEGIN`, `NS_ASSUME_NONNULL_END`, `NS_DESIGNATED_INITIALIZER`, `NS_REQUIRES_SUPER`, `NS_UNAVAILABLE`, `__attribute__((…))`; `typedef NS_ENUM(T, Name)` и `NS_OPTIONS` переписать в `enum Name : T` с выравниванием пробелами. Байтовые смещения и номера строк не меняются.
- **tree-sitter-objc: методы.** `method_definition` и `method_declaration` без полей, дети позиционные: `method_type`, затем чередуются `identifier` (кусок селектора) и `method_parameter`. Селектор — склейка `кусок:` для каждого `identifier`, за которым идёт параметр; для метода без аргументов — одиночный `identifier`. Знак `-`/`+` — анонимный токен. `.h` разбирать грамматикой ObjC.
- **gix 0.90: чего нет.** Нет фильтра по пути у `rev_walk`, нет Bloom-фильтров, нет аналогов `log -L` и `--follow`, в blame нет ignore-revs и поиска копий. Поэтому:
  - история пути: обход коммитов с пропуском тех, где `tree.lookup_entry_by_path(path)` даёт тот же OID, что у родителя; полный диф дерева с переименованиями — только когда путь пропал;
  - быстрый путь: если в системе есть `git`, список коммитов файла берётся из `git log --format=%H --full-history -- <path>` (фича `git-cli-fastpath`, по умолчанию включена) и кешируется;
  - `log -L`, `--follow` и учёт рефакторингов — собственная реализация (это и есть ядро работы).
- **gix 0.90: что работает.** `repo.blame_file(path, head, Options { ranges: BlameRanges::from_one_based_inclusive_range(5..=9)?, rewrites: Some(Rewrites::default()), .. })` даёт результат, совпадающий с `git blame -L5,9`, включая переименование файла. `Rewrites::limit` по умолчанию 1000: при превышении нечёткий поиск переименований отключается полностью — поэтому E5. Блоб по пути: `commit.tree()?.lookup_entry_by_path(p)?` → `entry.object()?.data`.
- **gix и async.** `Repository` — `Send`, но не `Sync`. В MCP-сервере держать `gix::ThreadSafeRepository`, в обработчике — `.to_thread_local()` внутри `tokio::task::spawn_blocking`.
- **imara-diff 0.2.** `let input = InternedInput::new(before, after); let mut d = Diff::compute(Algorithm::Histogram, &input); d.postprocess_lines(&input); for h in d.hunks() { … }`. `h.before` и `h.after` — 0-based диапазоны строк; между ханками номер строки переносится со сдвигом.
- **AST-диф.** Зрелого GumTree на Rust нет; difftastic — только бинарь, не библиотека. Сопоставление операторов (E7) пишется своё. `cinereus` (небольшой GumTree-крейт) можно использовать для сравнения в экспериментах. `sem-core` (диф на уровне сущностей, поддерживает Swift) — проверить как родственную работу.
- **rmcp 3.9.** Поддерживает MCP 2026-07-28 и старое рукопожатие `initialize` 2025-11-25. Инструменты: `#[tool_router]`, `#[tool(name, description)]`, `#[tool_handler]`; функция `async fn code_history(&self, Parameters(r): Parameters<Req>) -> Result<Json<Resp>, String>` сама даёт `inputSchema`, `outputSchema` и `structuredContent`. Переопределить `get_info`, иначе имя сервера будет `rmcp`. В stdout — только JSON-RPC, все логи — в stderr.
- **Tauri 2.** Команды — `#[tauri::command]` + `invoke`, аргументы из TypeScript в camelCase. Перетаскивание файла — `getCurrentWebview().onDragDropEvent`, в `payload.paths` полные пути. Большие ответы — через `tauri::ipc::Channel`.
- **CodeMirror 6, не Monaco.** 50–200 КБ против 2–5 МБ и веб-воркеров. Подсветка диапазона — `StateField<DecorationSet>` с `Decoration.line`, метки коммитов — свой `gutter`, режим только чтения — `EditorView.editable.of(false)`.

**Кеш.** Файл `<git-common-dir>/swhy/cache.sqlite`: внутри `.git`, общий для всех worktree, в репозиторий не попадает.

| Таблица | Ключ | Что хранит |
| --- | --- | --- |
| `parse` | OID блоба + `PARSER_VERSION` | символы, якоря, канонические хеши файла (bincode) |
| `symbol_history` | стартовый коммит + ключ символа + путь + `ALGO_VERSION` | события символа |
| `line_origin` | коммит + путь + хеш якоря | цепочка происхождения строки |
| `commit_info` | SHA + `CLASSIFIER_VERSION` | тип коммита, уверенность, задачи |
| `tasks` | ключ задачи | JSON задачи, время загрузки (TTL 24 ч) |
| `regressions` | SHA фикса C | найденные связи A → B → C |
| `test_links` | хеш якоря + ID теста | уровень, `verified_at`, версия Xcode |

Коммиты неизменяемы, поэтому записи с ключом по SHA не устаревают. После rebase или force-push они просто перестают использоваться; `swhy cache gc` удаляет записи недостижимых коммитов. Любое изменение алгоритма — повышение соответствующей `*_VERSION`, старые записи игнорируются.

**Целевая производительность** (MacBook на Apple Silicon, репозиторий около 50 000 коммитов): первый запрос по диапазону ≤ 10 с, повторный ≤ 200 мс, разбор одного файла ≤ 20 мс.

## Интерфейсы

Три входа (CLI, MCP, приложение) вызывают одну функцию ядра `why_core::query(Request) -> Response`. Разной у них только упаковка ответа.

**CLI:**

| Команда | Что делает | Код выхода |
| --- | --- | --- |
| `why <file> <start>-<end>` | История строк. Флаги: `--rev <rev>` (по умолчанию `HEAD`), \`--format compact | full |
| `why symbol <SymbolKey>` | История символа целиком | как выше |
| `why protected <file> [<start>-<end>]` | Только защитные строки | как выше |
| `why tests <file> <start>-<end> [--verify]` | Тесты строк; `--verify` запускает уровни `covered` и `guarding` | как выше |
| `why guard [--staged \| --diff <a>..<b>]` | Проверка изменений: задеты ли защитные строки, падают ли связанные тесты | 0 — чисто, 1 — задеты защитные строки, 3 — ошибка |
| `why cache stats \| clear \| warm <path>` | Управление кешем. `warm` — единственный способ посчитать заранее | 0 |
| `why mcp` | MCP-сервер по stdio | — |

**Конфиг `.why.toml` в корне репозитория:**

```toml
[tracker]
kind = "jira"                     # jira | github | none
base_url = "https://jira.example.com"
key_pattern = "[A-Z][A-Z0-9]+-\\d+"
# токен берётся из переменной окружения WHY_TRACKER_TOKEN, в файле не хранится

[tests]
workspace = "App.xcworkspace"
scheme = "AllTests"
destination = "platform=iOS Simulator,name=iPhone 16"

[ignore]
revs_file = ".git-blame-ignore-revs"           # коммиты, которые всегда технические
generated = ["**/Generated/**", "**/*.generated.swift", "**/R.generated.swift"]

[summary]
llm = false                       # сжатие описаний задач через LLM, по умолчанию выключено
```

**MCP-инструменты** (сервер `why mcp`, транспорт stdio):

| Инструмент | Параметры | Возвращает |
| --- | --- | --- |
| `code_history` | `file`, `line_start`, `line_end`, `include_diffs` (false), `max_events` (20) | Ответ `compact` |
| `protected_lines` | `file`, `line_start?`, `line_end?` | Только `protected_lines` |
| `related_tests` | `file`, `line_start`, `line_end`, `verify` (false) | Тесты с уровнями и командами запуска |
| `check_change` | `diff` (unified diff) или `staged: true` | Список задетых защитных строк и результат связанных тестов |

Описание каждого инструмента — 2–4 предложения: когда вызывать (перед правкой чужого кода, перед удалением «лишней» проверки), что вернёт и сколько это примерно токенов. Ответ отдаётся и как `structuredContent`, и как текстовый JSON для клиентов без structured output.

**Схема ответа `why/1` (формат `compact`):**

```json
{
  "schema": "why/1",
  "file": "Cards/CardViewModel.swift",
  "lines": [42, 58],
  "rev": "a1b2c3d",
  "symbols": ["swift:CardViewModel.reloadCards()"],
  "events": [
    {"date": "2025-09-18", "commit": "4f2e1aa", "type": "bugfix", "kind": "body_changed",
     "task": "IOS-7325", "title": "Fixes UI Bugs on feature 7325",
     "summary": "Список карт мигал при обновлении", "lines": [51], "caused": ["IOS-7391"]},
    {"date": "2025-10-02", "commit": "9c0d77e", "type": "bugfix", "kind": "body_changed",
     "task": "IOS-7391", "title": "Network crash on cache invalidation",
     "summary": "Запрос уходил с id из инвалидированного кеша", "lines": [51, 52]}
  ],
  "moves": [
    {"date": "2024-06-11", "kind": "cross_language", "from": "Legacy/CardsController.m", "confidence": "medium"},
    {"date": "2025-02-03", "kind": "moved_module", "from": "App", "to": "Cards"}
  ],
  "protected_lines": [
    {"line": 51, "protects": "IOS-7391", "reason": "сетевой краш при инвалидации кеша",
     "broke_before": ["IOS-7325"],
     "tests": [{"id": "CardsTests/CardViewModelTests/test_reload_afterCacheInvalidation",
                "strength": "guarding", "run": "xcodebuild test ... -only-testing:..."}]}
  ],
  "skipped": {"formatting": 4, "moves": 2},
  "truncated": null,
  "warnings": []
}
```

**Правила формата:**

- Ключи и массивы в фиксированном порядке: события по дате по возрастанию, защитные строки по номеру строки. Без времени генерации и без абсолютных путей.
- Технические коммиты не перечисляются, только считаются в `skipped`.
- `summary` задачи — не длиннее 160 символов; при `llm = false` это первое предложение описания.
- Бюджет ответа `compact` — до 1500 токенов (оценка: длина в байтах / 3,5). При превышении сначала обрезаются самые старые события не-багфиксы, потом `summary`. Что обрезано — в `truncated`.
- `full` добавляет к событиям `diff` только по запрошенным строкам (до 30 строк на событие).
- Ошибки: `{"schema":"why/1","error":{"code":"not_tracked","message":"…"}}`. Коды: `bad_range`, `not_tracked`, `not_a_repo`, `unsupported_language`, `internal`.

## Этапы E0–E3: основа

### E0. Каркас (неделя 1)

**Шаги:**

1. Создать workspace по структуре из раздела «Архитектура и стек». Лицензия MIT OR Apache-2.0, `THIRD_PARTY.md` для заимствованных списков слов из компилятора Swift.
2. CI (GitHub Actions, `macos-latest`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`.
3. Фреймворк fixture-репозиториев: `fixtures/lib.sh` с функциями `fx_init`, `fx_commit <msg> <date>`, `fx_write <path> <<EOF`. Автор, почта и даты фиксированы (`GIT_AUTHOR_DATE`, `GIT_COMMITTER_DATE`) — SHA детерминированы. Rust-хелпер `fixtures::build("fx-rename") -> PathBuf` собирает репозиторий в `target/fixtures/` один раз за прогон тестов.
4. Заготовки `docs/progress.md`, `docs/decisions.md`, `docs/oracle-guide.md`.
5. `swhy --version` и `swhy mcp` с пустым списком инструментов.

**Критерии приёмки:**

- [ ] CI зелёный; `fixtures::build` дважды даёт одинаковые SHA.

### E1. Датасет, замер проблемы и эталон (недели 1–2, разметка — неделя 10)

**Сужение датасета (решение @danilpapa, 2026-11-14).** E1 выполняется на одном репозитории — Signal-iOS. Пользователь уже склонировал его в `data/repos/Signal-iOS`: каталог не удалять и заново не клонировать. Остальные репозитории из таблицы ниже не клонируются и в критерии приёмки не входят. Где в этом этапе написано «для всех репозиториев» или «из 5 репозиториев», читать «для Signal-iOS»:

- `eval/repos.toml` закрепляет SHA HEAD только Signal-iOS;
- замер проблемы (пункты 1–4) — только на Signal-iOS;
- эталон: 50 методов из Signal-iOS; остальные условия эталона без изменений.

Этапы после E1, которые ссылаются на другие репозитории, это решение не меняет.

**Репозитории** (клонировать полностью в `data/repos/`, в git не коммитить; SHA HEAD закрепить в `eval/repos.toml`):

| Репозиторий | Роль |
| --- | --- |
| Signal-iOS | больше всего переписываний ObjC → Swift (153 пары файлов в одном коммите); почти нет ссылок на задачи |
| WordPress-iOS | история 2008–2026, около 81 000 коммитов, миграция идёт до сих пор, номера PR в сообщениях, переезды в SPM `Modules`; тест производительности |
| wikipedia-ios | переписывания с тем же именем и замены другими компонентами, переименования модулей |
| element-ios | смешанный код (325 `.m` и 2015 `.swift`), коммиты «convert X to swift + tests», ссылки на PR |
| firefox-ios | лучший пример модуляризации (BrowserKit, поглощённый focus-ios), 1226 тестовых файлов, стиль `FXIOS-#### (#PR)` |
| woocommerce-ios | переезды таргетов в SPM-пакеты, код Sourcery — проверка фильтра сгенерированных файлов |
| kickstarter/ios-oss | небольшой, чистый Swift, один коммит перенёс 1502 файла в SPM — быстрый тест переездов |
| Telegram-iOS (опционально) | только масштаб и устойчивость: Bazel, плохие сообщения коммитов |

Не брать: artsy/eigen (теперь React Native), duckduckgo/iOS (заморожен в 2025), focus-ios (вошёл в firefox-ios).

**Замер проблемы (N6, до появления инструмента)** — скрипты `eval/measure/*`:

1. Статистика репозитория: коммиты, годы, число файлов Swift и ObjC, коммиты с изменением больше 1000 файлов.
2. Шум blame: 500 случайных строк Swift из HEAD. Для каждой — коммит из `git blame -w` и из `git blame -w -M -C -C`. Доля строк, где коммит технический: изменение только пробелов (`git diff -w` пуст) или массовый коммит больше 200 файлов.
3. Обрывы истории: 200 случайных методов. История `git log -L :<func>:<file>` заканчивается на коммите, где файл добавлен. Если в том же коммите удалён файл с методом того же имени — это обрыв на переезде. Доля обрывов.
4. Грубая доля коммитов-багфиксов по ключевым словам (уточняется в E8, E11).

**Эталон истории методов** (по протоколу CodeShovel и HistoryFinder), правила разметки — в `docs/oracle-guide.md`:

1. 50 методов из 5 репозиториев, по 10 из каждого, у каждого не меньше 3 изменяющих коммитов. Не меньше 10 с переездом между модулями и не меньше 10 с ObjC-происхождением.
2. 25 методов для подбора порогов, 25 для проверки. Проверочную часть не смотреть до E17.
3. Кандидаты собираются объединением: `git log -L`, `git log --follow`, текущая версия инструмента. Затем ручная проверка каждого коммита.
4. Для каждого коммита: изменил ли он метод, класс (`FormatOnly`, `RenameOnly`, `Refactoring`, `Semantic`, переезд), коммит появления.
5. Решения заранее: изменение doc-комментария — отдельный технический класс `DocOnly`; изменение атрибутов (`@MainActor`, `@objc`) — `SignatureChanged: attributes`; merge-коммит сравнивается с каждым родителем.
6. Второй разметчик проверяет 30% методов; посчитать каппу Коэна.
7. Формат: `eval/oracle/methods/<repo>-<n>.json`.
8. На неделе 10 — построчный эталон: 20 методов из проверочной части, около 400 строк, у каждой строки — последний смысловой коммит.

**Критерии приёмки:**

- [ ] `eval/results/E1-measure.json` с цифрами пунктов 1–4 для Signal-iOS.
- [ ] 50 размеченных методов, каппа посчитана.

### E2. Разбор кода и ключи символов (неделя 3)

**Шаги:**

1. `parse/swift.rs`: обход дерева tree-sitter-swift. Все символы из таблицы «Формат ключа символа»: типы по `declaration_kind`, члены, вложенные типы, члены extension приписываются расширяемому типу, глобальные функции. Протокольные требования — отдельные символы вида `Type`.
2. `#if`: символы между соседними `directive` получают пометку условной компиляции; две реализации под разными ветками различаются суффиксом `#n`.
3. Символ с `node.has_error()` — признак `degraded`, для него E7 переключается на LHDiff.
4. `parse/objc_prepass.rs` и `parse/objc.rs`: предобработка, `@interface`, `@implementation`, категории, селекторы, `-`/`+`, `.h`.
5. Для каждого символа: `lines`, `signature_hash`, `body_hash` (до E6 — по тексту без пробелов и комментариев; после — по канонической форме), якоря операторов (путь в дереве + хеш), скелет для E9.
6. Результат разбора блоба — в кеш `parse` по OID.
7. Замер на HEAD всех репозиториев E1: доля файлов без `ERROR`, время разбора на файл.

**Критерии приёмки:**

- [ ] Табличный тест на каждый пример из таблицы ключей и на все подводные камни tree-sitter из раздела стека (поле `name` у типов, subscript без имени, extension в другом файле, `#if`).
- [ ] Доля Swift-файлов без `ERROR` ≥ 98% (иначе — запасной вариант из «Рисков»); медиана разбора ≤ 20 мс на файл.

### E3. Слой git и история строк v0 (неделя 4)

**Шаги:**

1. `git/repo.rs`: открытие через `gix::ThreadSafeRepository`, `blob_at(rev, path)`.
2. `git/walk.rs`: история пути — обход коммитов с пропуском тех, где OID записи по пути не изменился относительно родителя; быстрый путь через `git log --format=%H --full-history -- <path>`; merge-коммиты — по каждому родителю.
3. Путь пропал — `diff_tree_to_tree` с `Rewrites { percentage: Some(0.5), limit: 0 /* без лимита только для этого коммита */, .. }`, чтобы найти новое имя файла. Если и так не нашлось — дело E5.
4. `git/diff.rs`: перенос номеров строк через ханки imara-diff (Histogram + `postprocess_lines`).
5. `swhy <file> <a>-<b>` v0: история диапазона строк только на уровне текста — аналог `git log -L` со своим переносом строк. Нужен как основа и базовая линия.
6. Кеш: таблица `parse` и список коммитов файла по SHA старта.

**Критерии приёмки:**

- [ ] На 100 случайных диапазонах из 3 репозиториев множество коммитов v0 совпадает с `git log -L <a>,<b>:<file>` (расхождения разобраны в `docs/decisions.md`).
- [ ] На WordPress-iOS: первый запрос ≤ 10 с, повторный ≤ 200 мс.

## Этапы E4–E7: отслеживание

Эти этапы дают N1 и N5. Основа — алгоритмы CodeShovel, HistoryFinder и CodeTracker для Java, перенесённые на Swift. Для Swift ни одного из них нет: RefactoringMiner, CodeShovel, CodeTracker и HistoryFinder Swift не поддерживают.

### E4. Отслеживание символа сквозь коммиты (недели 5–6)

**Цель:** по `SymbolKey` и пути в стартовой ревизии построить список событий символа до коммита, где он появился.

**Входы:** E2 (разбор и ключи), E3 (слой git, кеш разборов по OID блоба).

**Алгоритм** (схема HistoryFinder: BFS с обработкой merge-коммитов по каждому родителю):

1. Граф коммитов файла: коммиты, где менялся путь `P`, начиная со стартовой ревизии. Обход в ширину. Узел очереди — `(commit, path, key, lines)`.
2. Для узла взять дерево файла в коммите и в каждом родителе. Для merge-коммита сравнивать с каждым родителем отдельно. Если символ в родителе совпадает побайтово — идти по этому родителю без события.
3. Найти символ в родителе. Фазы по порядку, первая удачная побеждает:
   1. Тот же файл, тот же ключ. Хеши тела и сигнатуры равны — изменений нет. Иначе событие `BodyChanged` или `SignatureChanged` с подтипом (см. ниже).
   2. Тот же файл, тот же вид символа, сходство тел по Jaro-Winkler ≥ `T_same` (старт 0,70) — `Renamed` или `SignatureChanged`.
   3. Другие файлы, изменённые в коммите (удалённые, добавленные, изменённые): тот же ключ — `MovedFile`. Если ключ другой, сходство ≥ `T_other` (старт 0,75) — `MovedFile` + `Renamed`. Поиск кандидатов идёт через индекс из E5, а не перебором.
   4. Проверка на извлечение: если найденный кандидат в родителе — бо́льший метод, который в текущем коммите тоже существует, а тело символа похоже лишь на его часть, это не история символа. Символ помечается `Introduced` с пометкой `extracted_from`. Это закрывает известную ошибку CodeShovel.
   5. Ничего не нашлось — `Introduced`, обход ветки заканчивается.
4. Сходство считать на токенах канонической формы из E6. До готовности E6 — на тексте без пробелов и комментариев. Jaro-Winkler из крейта `strsim`.
5. Перегрузки различаются метками в ключе. Однофамильцы `#n` сопоставляются по порядку в файле.
6. Ответ — события в хронологическом порядке. Merge-коммит, через который символ прошёл без изменений, событием не считается.

**Подтипы `SignatureChanged`, специфичные для Swift** (вклад в новизну, в Java-инструментах их нет):

| Подтип | Пример |
| --- | --- |
| `labels` | `load(id:)` → `load(for:)` |
| `effects` | добавили `async`, `throws`, `rethrows`, typed throws |
| `attributes` | `@MainActor`, `@objc`, `@discardableResult`, `@available` |
| `access` | `internal` → `private`, `public` → `package` |
| `isolation` | `nonisolated`, `Sendable` в сигнатуре |
| `container` | метод переехал из тела класса в `extension` (ключ тот же, событие `MovedToExtension`) |

**Подбор порогов:** сетка от 0,50 до 0,95 с шагом 0,05 по `T_same` и `T_other` на обучающей части эталона E1. Критерий — F1 на уровне коммитов. Итоговые значения записать в `docs/decisions.md` и в константы с комментарием, откуда они.

**Критерии приёмки:**

- [ ] Fixture-репозиторий `fx-rename` (переименование, смена меток, перенос в extension в другом файле, перегрузки, merge-коммит) даёт ровно ожидаемые события.
- [ ] На тестовой части эталона: F1 на уровне коммитов ≥ 0,90; базовые линии `git log -L <a>,<b>:<file>` и `git log -L :<func>:<file>` посчитаны на тех же методах.
- [ ] Медиана времени на символ без кеша ≤ 3 с, с кешем ≤ 200 мс.

### E5. Переезды и гигантские коммиты (неделя 6)

**Цель:** находить символ после коммита, который двигает тысячи файлов, не опираясь на детектор переименований git. У git `diff.renameLimit` по умолчанию 1000 файлов, а порог сходства — 50%: при модуляризации переименования молча теряются.

**Шаги:**

1. Чистые переезды: пары «удалён путь A, добавлен путь B» с одинаковым OID блоба сопоставляются за O(1) через хеш-таблицу OID → путь.
2. Индекс отпечатков символов коммита. Для всех изменённых файлов коммита (параллельно через rayon, разборы из кеша) собрать:
   - точную карту `body_hash → [символы]`;
   - MinHash по 3-граммам канонических токенов (64 перестановки) и LSH-корзины (16 полос по 4) для приблизительного поиска.
3. Поиск символа в родителе: сначала точный хеш тела, затем кандидаты из LSH-корзин, затем проверка Jaro-Winkler с порогом `T_other`. Сложность — линейная от числа символов коммита, без квадратичного перебора файлов.
4. Модуль файла:
   - если в `.why.toml` есть `[modules]` с правилами путей — по ним;
   - иначе ближайший предок пути вида `<X>/Sources/<Module>/` (SPM, Tuist) или каталог с `Package.swift`, `Project.swift`, `*.xcodeproj`. Если модуль у символа до и после разный — событие `MovedModule { from_module }` вместо `MovedFile`.
5. Разбиение файла: символы одного старого файла разъехались по нескольким новым — у каждого символа событие `MovedFile`, плюс на уровне файла запись `split` для показа в `moves`.

**Критерии приёмки:**

- [ ] Fixture `fx-modularize`, создаваемый скриптом: 3000 файлов переносятся одним коммитом в 12 модулей, в 20 файлах одновременно правятся тела. Все 20 символов находятся, события `MovedModule` верные.
- [ ] Время одного запроса через этот коммит ≤ 10 с без кеша.
- [ ] На реальных репозиториях из E1: доля символов, чья история прошла через коммит с более чем 1000 изменёнными файлами, без обрыва — отчёт в `eval/results`.

### E6. Каноническая форма Swift и классификация изменений (неделя 7)

**Цель:** отделить смысловые изменения от технических (N5). Сравнение двух версий символа даёт один из классов: `FormatOnly`, `RenameOnly`, `Refactoring`, `Semantic`.

**Каноническая форма** — поток токенов по дереву tree-sitter, без комментариев и пробелов, с правилами:

| Правило | До | Каноническая форма |
| --- | --- | --- |
| Явный `self.` | `self.items.count` | `items.count` |
| Сокращённый unwrap | `if let card = card` | `if let card` |
| Скобки в условии | `if (a > b)` | `if a > b` |
| Trailing closure | `foo(x, completion: { … })` | `foo(x) { … }` |
| Пустые скобки перед trailing closure | `items.map() { … }` | `items.map { … }` |
| Неявный `return` | `{ return x * 2 }` | `{ x * 2 }` |
| Висячая запятая | `[a, b,]` | `[a, b]` |
| `;` в конце строки | `let a = 1;` | `let a = 1` |
| Порядок членов типа | любые перестановки | множество ключей, порядок не важен |
| Порядок `import` | любой | отсортирован |

**Классы:**

- `FormatOnly` — канонические формы равны, отличаются только пробелы, переносы, комментарии.
- `RenameOnly` — канонические формы равны после согласованного переименования локальных идентификаторов (альфа-переименование: локальные имена заменяются на `$1`, `$2` по порядку объявления) и переименования самого символа.
- `Refactoring` — узнанное структурное преобразование без смены поведения: `if let` ↔ `guard let`, перенос в extension, вынос выражения в локальную константу (только если константа используется один раз).
- `Semantic` — всё остальное. Отдельно помечается подтип `concurrency`: добавление `@MainActor`, `Sendable`, `nonisolated`, `async`. Для истории это смысловое изменение, а не косметика.

**Технический коммит** — у которого все события в запрошенных строках `FormatOnly`, `RenameOnly` или переезды. Такие коммиты скрываются и считаются в `skipped`.

**Дополнительно:** команда `why ignore-revs` выводит список коммитов, которые по всему репозиторию только форматирование, в формате `.git-blame-ignore-revs`. Так обычный `git blame` тоже начинает пропускать их.

**Критерии приёмки:**

- [ ] Property-тест: для каждого Swift-файла fixture `canon(swift_format(f)) == canon(f)`. Прогнать через `swift-format` и `swiftformat` с настройками по умолчанию.
- [ ] Ручная разметка 200 изменений из E1 (класс до и после): точность класса «технический» ≥ 0,95, полнота ≥ 0,85.
- [ ] Ни одно изменение с подтипом `concurrency` не попадает в технические.

### E7. Атрибуция строк внутри символа (недели 8–9)

**Цель:** для каждой строки запрошенного диапазона найти цепочку её версий до появления и последний смысловой коммит. Это то, что агент видит как «строка 51 пришла из IOS-7391».

**Уровни сопоставления операторов** между версией символа в родителе и в коммите, по порядку:

1. **Быстрый путь по ханкам.** Диф файла (imara-diff, алгоритм Histogram). Строки вне ханков переносятся сдвигом номера. Это покрывает большинство коммитов.
2. **Точное совпадение листовых операторов** по хешу канонического текста.
3. **Совпадение после замен** (идея RefactoringMiner): операторы считаются одним, если их токены отличаются только заменами идентификаторов и литералов, согласованными с картой переименований символа. Ничьи разрешаются по расстоянию редактирования, глубине вложенности и индексу среди соседей.
4. **Составные операторы** (`if`, `guard`, `for`, `while`, `switch`, `do`, `defer`, замыкания): по доле совпавших потомков (коэффициент Дайса ≥ 0,5), как bottom-up фаза GumTree.
5. **Запасной путь — LHDiff** для оставшихся строк, комментариев и неразобранных мест: нормализованное содержимое плюс контекст 4 строки выше и 4 ниже, 64-битный SimHash, 15 кандидатов на строку, оценка 0,6·содержимое + 0,4·контекст, принять при > 0,45. Склейка разбитой строки — до 8 строк, порог 0,85.

**Якорь оператора** — по схеме CodeTracker для блоков: вид оператора + цепочка родителей с индексами детей + хеш тела. Пример: `[Body, Stmt(3), IfThen, Stmt(0)]`.

**Миграции блоков Swift**, которые распознаются как `Refactoring` с переносом якорей внутрь:

- `if let` ↔ `guard let`;
- `for-in` ↔ `forEach` / `map`;
- цепочка `if-else` ↔ `switch`;
- completion handler ↔ `async`/`await` (тело замыкания ↔ код после `await`).

**Результат:** для каждой строки HEAD — список `(commit, строка в той ревизии, класс изменения)` до появления. «Происхождение» строки — последний коммит с классом `Semantic`.

**Критерии приёмки:**

- [ ] Fixture `fx-lines`: вставка строк выше метода, перестановка метода, `swiftformat` по всему файлу, `if let` → `guard let`, правка одной строки. Происхождение каждой строки верное.
- [ ] Построчный эталон (20 методов из E1, примерно 400 строк): точность последнего смыслового коммита ≥ 0,85. Та же метрика для `git blame -w -M -C` как базовая линия.

## Этапы E8–E11: смысл истории

Эти этапы дают N2 и половину N6. Готовые подходы (SZZ, классификация коммитов, связь с задачами) есть, но ни один не проверялся на Swift. Инструмента, который показывает «эта строка защищает от бага X», в известной литературе не найдено.

### E8. Тип коммита и задачи из трекера (недели 10–11)

**Цель:** у каждого коммита в истории — тип (`Bugfix`, `Feature`, `Refactoring`, `Formatting`, `Test`, `Build`, `Merge`, `Unknown`), уверенность и связанные задачи.

**Сигналы для типа**, от сильного к слабому:

| Сигнал | Как получить | Вес |
| --- | --- | --- |
| Тип задачи в трекере: Bug, Defect, Incident | Jira `issuetype.name`, GitHub — метки `bug`, `regression` | сильный |
| Структура изменения из E6 | все изменения `FormatOnly` → `Formatting`; канонически равны → `Refactoring` | сильный, решающий для этих двух классов |
| Затронутые файлы | только тесты → `Test`; только `Package.swift`, `Project.swift`, `*.pbxproj`, `Podfile`, `*.xcconfig` → `Build` | сильный |
| Префикс Conventional Commits | `fix:`, `fix(scope):`, `hotfix:`, `feat:`, `refactor:`, `test:`, `build:`, `chore:` | средний |
| Имя ветки | `bugfix/`, `hotfix/`, `fix/` из сообщения merge-коммита или `head.ref` PR | средний |
| Ключевые слова в сообщении | `fix`, `bug`, `crash`, `regression`, `hotfix`, `EXC_BAD_ACCESS`, `leak`, `retain cycle`, `force unwrap`, `main thread`, `deadlock`, `race`, `nil`, `краш`, `баг`, `исправ`, `падени` | слабый |

**Решение:** таблица правил в `why-core/src/classify.rs` в указанном порядке. Первое сработавшее сильное правило задаёт тип с `High`. Только средние и слабые сигналы — `Medium` или `Low`. Противоречие (Jira говорит Bug, а изменение `FormatOnly`) — побеждает структура, в `warnings` пишется причина. Опционально, при `[summary] llm = true`: для `Unknown` LLM получает сообщение и диф до 200 строк и возвращает один класс.

**Связь коммита с задачей:**

1. Регулярное выражение `key_pattern` из `.why.toml` по сообщению коммита, имени ветки из merge-коммита и заголовку PR.
2. GitHub: `GET /repos/{owner}/{repo}/commits/{sha}/pulls` → PR (работает для коммитов из основной ветки); затем GraphQL поле `closingIssuesReferences` PR и ссылки `#123` в теле PR.
3. Jira Cloud: `GET /rest/api/3/issue/{key}?fields=summary,issuetype,status,created,description,issuelinks,labels`. Описание приходит в Atlassian Document Format — извлечь текст обходом узлов `type: "text"`. Jira Data Center: `GET /rest/api/2/issue/{key}`, заголовок `Authorization: Bearer <PAT>`. Токен — из `WHY_TRACKER_TOKEN`.
4. Учитывать лимиты: не больше 5 параллельных запросов, при `429` ждать `Retry-After`.
5. Кеш задач в SQLite с TTL 24 часа (задачи меняются, коммиты — нет). Без сети ответ строится из кеша, в `warnings` — `tracker_offline`.
6. `summary` задачи — первое предложение описания или заголовок, не длиннее 160 символов.

**Критерии приёмки:**

- [ ] Ручная разметка 200 коммитов из 3 репозиториев E1: точность класса `Bugfix` ≥ 0,85, полнота ≥ 0,75. Та же метрика для одних ключевых слов — базовая линия.
- [ ] Отчёт: доля коммитов со связанной задачей в каждом репозитории.
- [ ] Интеграционный тест трекеров на записанных ответах (`wiremock` или `httpmock`), без реальной сети.

### E9. Переписывание ObjC → Swift (недели 12–13)

**Цель (N1):** когда история Swift-метода упирается в `Introduced`, найти ObjC-метод, который он заменил, и продолжить историю по нему. Исследований и датасетов именно для пары ObjC ↔ Swift не найдено; межъязыковые клоны изучали в основном для Java ↔ C#.

**Три формы миграции**, все обязательны:

| Форма | Как выглядит | Пример из реальных репозиториев |
| --- | --- | --- |
| A. Один коммит, то же имя | удалён `X.m` / `X.h`, добавлен `X.swift` | Signal-iOS — 153 таких пары, WordPress — 57, Wikipedia — 34 |
| B. Растянуто | `X.swift` добавлен, `X.m` удалён позже, несколько PR (`Convert Blog to Swift (P4)`) | WordPress-iOS |
| C. Замена другим типом | ObjC-класс заменён Swift-компонентом с другим именем в другом модуле | wikipedia-ios |

**Шаг 1. Кандидаты.** Для Swift-символа, появившегося в коммите S:

- форма A — ObjC-методы файлов с тем же базовым именем, удалённых в S;
- форма B — ObjC-методы, которые существовали на момент S и были удалены в течение 180 дней после S (или живы до сих пор — период сосуществования бывает годами);
- форма C — ObjC-методы, удалённые в окне ±180 дней в любом файле, через индекс отпечатков E5.

**Шаг 2. Предсказание Swift-имени по селектору** — безопасное подмножество правил Clang Importer (полные правила требуют типов). Генерировать множество вариантов имени, не одно:

| Правило | ObjC | Swift |
| --- | --- | --- |
| init-семейство: `With` в начале первой метки отбрасывается | `initWithName:age:` | `init(name:age:)` |
| Фабрика → init: первый кусок начинается с имени класса и метод возвращает `instancetype` или класс | `+[CardColor colorWithRed:]` | `init(red:)` |
| `NSError **` в конце → `throws`, суффиксы `AndReturnError`, `WithError` убираются | `saveAndReturnError:` | `save() throws` |
| Completion handler → `async`: последний параметр-блок с именем из списка (`completion`, `completionHandler`, `withCompletion`, `withBlock`, …), суффиксы убираются, ведущий `get` отбрасывается | `getCardsWithCompletion:` | `cards() async` (плюс вариант без async) |
| Первый кусок делится по **последнему** предлогу, `with` и `using` отбрасываются | `loadCardsForUser:` | `loadCards(for:)` |
| Bool-параметр с последним словом `Animated` → метка `animated` | `dismissAnimated:` | `dismiss(animated:)` |
| Аббревиатуры в начале — строчными | `URLForKey:` | `url(forKey:)` |
| Обрезка имени типа в хвосте — **только как дополнительный вариант** | `documentForURL:` | `document(for:)` и `documentFor(_:)` |
| Исключения деления по предлогу | `plugIn`, `withError`, `withNo`, `toVisible`, `toBacking`, `fromBacking` | не делить |

Списки предлогов и глаголов перенести из `PartsOfSpeech.def`, логику — из `lib/Basic/StringExtras.cpp` (`omitNeedlessWords`, `splitBaseNameAfterLastPreposition`) и `lib/ClangImporter/ImportName.cpp` компилятора Swift. Лицензия Apache-2.0 с Runtime Library Exception — сохранить уведомление в `THIRD_PARTY.md`. Готовой библиотеки нет: SourceKit `-req=translate` требует компилируемого контекста.

**Шаг 3. Точное обратное сопоставление** — сильнейший сигнал:

- у Swift-объявления есть `@objc(sel:)` — сравнить селектор напрямую;
- объявление `@objc` или член наследника `NSObject` — вычислить ObjC-селектор из Swift-имени по правилам `getObjCSelector` в `lib/AST/Decl.cpp` и сравнить с удалённым селектором точно. Примеры для тестов: `init(name:)` → `initWithName:`, `foo(bar:baz:)` → `fooWithBar:baz:`, `foo(_:baz:)` → `foo:baz:`, `foo()` → `foo`. Правила throws и async — сверить с исходником Decl.cpp.

Почему это работает: пока метод был на ObjC, Swift-код уже звал его по импортированному имени; при переписывании команды обычно сохраняют это имя, чтобы не трогать вызовы.

**Шаг 4. Сходство по скелету** — для формы C и когда имена не совпали. Скелет метода, без типов:

1. вызванные селекторы и методы, нормализованные правилами шага 2 (`[x foo:a bar:b]` → `foo:bar:` ↔ `x.foo(a, bar: b)`);
2. строковые литералы (`@"packId"` → `"packId"` — почти всегда копируются как есть);
3. числовые константы;
4. мешок слов из идентификаторов, разбитых по camelCase, строчными; без `self`, `NS`, `nil`; `YES`/`NO` → `true`/`false`;
5. упомянутые типы без префикса NS и с мостами (`NSString` → `String`, `NSArray` → `Array`, `NSDictionary` → `Dictionary`);
6. число параметров без error- и completion-параметра;
7. грубая форма управления: число `if`, `for`, `switch`, `return` (слабый сигнал: `guard` и optional chaining меняют форму).

Оценка: `0,35·Jaccard(селекторы) + 0,25·Jaccard(литералы) + 0,25·cos(слова) + 0,10·Jaccard(типы) + 0,05·[число параметров совпало]`. Бонус +0,1, если имя Swift-типа совпадает с именем ObjC-класса без префикса из 2–3 заглавных букв (`TSCardsController` → `CardsController`). Принять, если оценка ≥ порога (старт 0,5) и лучший кандидат опережает второго на ≥ 0,1. Веса и порог — подобрать на обучающих парах.

**Шаг 5. Событие и продолжение.** Создаётся `CrossLanguage { from, from_path }` с уверенностью: `High` — точное совпадение имени или селектора (шаги 2–3); `Medium` — предсказанное имя плюс скелет выше порога; `Low` — только скелет. Дальше E4 продолжает историю уже по ObjC-символу с ключом-селектором.

**Шаг 6. Строки.** По умолчанию строки через переписывание не переносятся: защитная строка из ObjC-части показывается на уровне метода («метод переписан с ObjC; в ObjC-версии был фикс IOS-…»). Перенос отдельных строк по общим литералам и селекторам — опция, в «дальнейшее развитие».

**Сгенерированные файлы** исключаются до поиска кандидатов: пути `Pods/`, `Carthage/`, `.build/`, `DerivedData/`, `*.generated.swift`, `/Generated/`; заголовок в первых 10 строках по регулярному выражению `generated (using|by)|autogenerated|auto-generated|do not edit|computer generated|@generated` (без учёта регистра); исключения для формулировок «originally generated», «manually updated»; атрибут `linguist-generated` в `.gitattributes`. Список правил — в `.why.toml`.

**Эталон:** «серебряный» — пары файлов формы A (Signal, WordPress, Wikipedia); «золотой» — 200 пар методов из Signal-iOS и WordPress-iOS, размеченных вручную, 100 для подбора весов и 100 для проверки.

**Критерии приёмки:**

- [ ] Fixture `fx-objc`: формы A и B с методами init, фабрикой, throws, completion → async и с `@objc(sel:)`. Все пары находятся с ожидаемой уверенностью.
- [ ] На тестовых парах: точность ≥ 0,80 при полноте ≥ 0,60.
- [ ] Таблица абляции для главы 4: только имена / только скелет / вместе.

### E10. Защитные строки (неделя 14)

**Цель:** для каждой строки запроса решить, защищает ли она от старого бага, и от какого.

**Определение.** Строка L в HEAD защитная, если в её цепочке происхождения (E7) есть событие класса `Semantic` из коммита типа `Bugfix`, и после него у строки были только события `FormatOnly`, `RenameOnly`, `Refactoring` или переезды.

**Шаги:**

1. Для каждой строки диапазона взять цепочку из E7.
2. Найти последнее событие `Semantic`. Если коммит этого события — `Bugfix`, строка защитная с уверенностью по типу коммита (`High`, если тип пришёл из трекера; `Medium`, если по сообщению).
3. Если последнее событие `Semantic` — не багфикс, но раньше в цепочке был багфикс, и текущий текст строки содержит те же ключевые токены, что внёс багфикс (пересечение множеств идентификаторов и литералов ≥ 0,7), — защитная с `Low`.
4. Не считать защитными: пустые строки, комментарии, `import`, одиночные `}`, строки только с логированием (`print`, `os_log`, `Logger.*`, `NSLog`; список настраивается).
5. Отдельно ловить строки, которые багфикс **добавил**, а не изменил: `guard`, проверки на `nil`, `[weak self]`, `DispatchQueue.main`, `@MainActor`. Классический SZZ их не видит, потому что ему нечего blame-ить. Для защитных строк это главный случай.
6. Причина (`reason`) — `summary` задачи багфикса. Нет задачи — первая строка сообщения коммита.
7. Строки одного багфикса подряд объединяются в один диапазон, чтобы не раздувать ответ.

**Ближайший аналог** для текста диплома: скрипт `find-regression-risk` в dotnet/maui. Он помечает PR, который удаляет строку, добавленную недавним багфиксом. Это точное сравнение строк на уровне PR: без blame, без учёта рефакторингов, без цепочек.

**Критерии приёмки:**

- [ ] Fixture `fx-protect`: багфикс добавляет `guard`, затем `swiftformat`, переименование метода и переезд в другой модуль. Строка остаётся защитной на каждом шаге.
- [ ] Ручная разметка 100 строк (примерно 50 защитных и 50 нет) из 3 репозиториев: точность ≥ 0,80, полнота ≥ 0,70.

### E11. Цепочки регрессий (неделя 15)

**Цель:** найти связи «фикс A внёс баг B, B исправлен фиксом C» и показать их в `caused` и `broke_before`.

**Алгоритм — SZZ на якорях.** Базовый вариант — R-SZZ с фильтром по дате задачи: на эталоне Rosa et al. он лучший из классических, F1 около 0,61–0,64. Новое — blame идёт не по тексту, а по цепочкам E7, поэтому переезды и форматирование не ломают его.

1. Считать лениво: только для багфиксов, которые встретились в истории запрошенных символов. Результат кешировать по SHA фикса.
2. Для багфикса C (задача B) взять строки, которые C удалил или изменил смысловым образом (класс `Semantic`). Если C только добавлял строки, кандидатами становятся соседние операторы того же блока (до 3 выше и ниже), с уверенностью `Low`.
3. Для каждой строки-кандидата найти происхождение через E7, пропуская технические коммиты. Это и есть кандидаты в коммит, внёсший баг.
4. Отбросить кандидатов, сделанных после создания задачи B (если дата есть).
5. Из оставшихся выбрать самый поздний (правило R-SZZ).
6. Если выбранный коммит — `Bugfix` с задачей A, создаётся `RegressionLink { fix_a, task_a, bug_b, fix_c, shared_anchors }`.
7. Повысить уверенность до `High`, если есть явное подтверждение: связь задач в Jira вида «is caused by» / «causes», метка `regression`, сообщение с `regression from <key>`, `introduced in <sha>`, `Fixes: <sha>`, `caused by #123`. Если такая связь есть, а SZZ её не нашёл, — добавить с `Medium`.
8. Цепочки длиннее двух звеньев собираются транзитивно.

**Эмпирика для N6:** на 6–8 репозиториях E1 посчитать долю багфиксов, у которых найденный внёсший баг коммит — сам багфикс. Сравнить с Yin et al. (14,8–24,4% неверных фиксов) и Park et al. (22,5–32,8% багов требовали больше одного фикса).

**Эталон.** Публичного набора пар «фикс — внёсший баг коммит» для Swift нет. Собрать свой по методу Rosa et al.: в репозиториях E1 найти коммиты с сообщениями `introduced in`, `regression`, `Fixes: <sha>`, `caused by`, `reverts`. Двое разметчиков проверяют 100–200 пар. Это первый такой эталон для Swift — отдельный вклад.

**Критерии приёмки:**

- [ ] Fixture `fx-chain`: фикс A меняет индекс → коммит B исправляет краш, вызванный A → цепочка находится и после переезда метода в другой файл между A и B.
- [ ] На эталоне: точность и полнота связей «внёс баг». Сравнение двух вариантов: обычный blame (B-SZZ) и наш blame с учётом рефакторингов. Разница показывает эффект N1.

## Этапы E12–E15: тесты, защита, интерфейсы

### E12. Связь защитных строк с тестами (недели 16–17)

**Цель (N3):** у каждой защитной строки — список тестов с силой связи и готовой командой запуска.

**Шаг 0. Спайк на реальном Xcode 26 (обязателен, полдня).** Проверить и записать в `docs/decisions.md`:

- форму JSON `xcrun xccov view --archive --json --file <path> <bundle.xcresult>` (`xcrun xccov help view`). Ожидаемо: `{ "<абсолютный путь>": [ {"line": N, "isExecutable": bool, "executionCount": K, ...} ] }`, но это не проверено;
- как `-only-testing` принимает тесты Swift Testing: `Target/Suite/func()` или `Target/Suite/func()()` (в некоторых сборках Xcode 16 нужны двойные скобки);
- что выдаёт `xcrun xcresulttool get test-results tests --path X.xcresult --compact`;
- даёт ли покрытие ненулевые числа для локальных SPM-пакетов (на Xcode 26 бывают жалобы на 0%).

**Поиск тестов.** Разбор тестовых файлов через tree-sitter:

- XCTest: классы, унаследованные от `XCTestCase` прямо или через базовый класс из того же репозитория (иерархия строится внутри тестовых таргетов), методы `func test…()` без параметров;
- Swift Testing: функции с атрибутом `@Test`, типы с `@Suite`.

Канонический ID теста: `<таргет>/<путь сьюта через />/<функция с метками>`, например `CardsTests/CardViewModelTests/test_reload()`. Таргет — по правилам модулей E5. Тесты отслеживаются тем же ядром E4: переименование теста или переезд в другой модуль связь не ломают. Окончательные ID сверяются с `xcresulttool` после первого прогона.

**Уровень `history` (бесплатно, только git):**

1. Взять багфикс F, который внёс защитную строку.
2. Тесты, добавленные или изменённые в F (по разбору дифа тестовых файлов).
3. Расширить: остальные коммиты того же PR (GitHub) и коммиты с тем же ключом задачи в пределах 14 дней.
4. Эвристика имён (`CardViewModelTests` ↔ `CardViewModel`, `test_reloadCards` ↔ `reloadCards()`) только поднимает тест выше в списке, связь сама не создаёт: в исследованиях одно совместное изменение — слабый сигнал.

**Уровень `covered` (нужен прогон, только по запросу `--verify`):**

1. Кандидаты: тесты `history` ∪ тесты, совпавшие по именам ∪ тесты тестового таргета модуля строки. Не больше 30 (настраивается). Весь набор не гоняется никогда.
2. Один раз собрать: `xcodebuild build-for-testing -workspace W -scheme S -destination D -derivedDataPath <кеш>/dd -enableCodeCoverage YES`.
3. Для каждого кандидата: `xcodebuild test-without-building -xctestrun <кеш>/dd/Build/Products/*.xctestrun -destination D -only-testing:<id> -enableCodeCoverage YES -parallel-testing-enabled NO -resultBundlePath <кеш>/cov/<slug>.xcresult`.
4. `xcrun xccov view --archive --json --file <абсолютный путь файла> <bundle>` — строка покрыта, если `isExecutable && executionCount > 0`. Абсолютные пути привести к путям от корня репозитория.
5. Для чистых SPM-пакетов (работает и на Linux): `swift test --enable-code-coverage --filter <id>`, затем `llvm-cov export`.
6. Ускорение одним прогоном (наблюдатель `XCTestObservation`, который сбрасывает счётчики LLVM перед каждым тестом) — только после спайка и только как опция. Swift Testing такого хука не имеет.

**Уровень `guarding` (самый дорогой, только по запросу):**

1. Взять ханк багфикса F по защитной строке и перенести его на HEAD через якоря E7. Если строка с тех пор изменилась смыслово — связь `stale`, откат не делается.
2. Создать отдельный `git worktree` со своим DerivedData. Рабочее дерево и DerivedData пользователя не трогаются.
3. В worktree заменить текущий текст строк на текст до фикса (обратное применение ханка `-U0`).
4. `build-for-testing` инкрементально. Не собралось — результат `revert_uncompilable`, это не `guarding`.
5. Кандидаты = `history` ∪ `covered`. Тест, который не проходит через строку, откат поймать не может.
6. Прогнать кандидатов на HEAD и на мутанте, без покрытия, последовательно, с таймаутом на тест (`-test-timeouts-enabled YES`). Каждый по 3 раза.
7. `guarding` = тест стабильно зелёный на HEAD и стабильно красный на мутанте. Нестабильный на HEAD — помечается `flaky` и не используется.
8. Удалить worktree.

**Устаревание:** при запросе, если файл строки или файл теста менялся после `verified_at`, связь показывается как `stale`. Пересчёт — только с `--verify`.

**Правило для агента** (входит в описание MCP-инструмента): при правке строк сначала запустить `guarding` и `history` тесты затронутых защитных строк, затем `covered`.

**Как формулировать новизну.** Откат фиксов для получения реальных ошибок известен (Just et al. 2014, Defects4J, RegMiner). Проверка, что тест фикса падает на коде до фикса, тоже есть, но откатывается весь PR. В просмотренных работах и инструментах не найдено сочетание: откат **одного ханка** исторического фикса **на HEAD**, проверка **любого текущего теста** и сохранение результата как связи «строка → тест», которую агент видит перед правкой. Формулировать как «не найдено в рассмотренных работах», не «никто не делал».

**Критерии приёмки:**

- [ ] Fixture `fx-tests` — SPM-пакет с XCTest и Swift Testing (запускается `swift test` без симулятора). Багфикс добавляет `guard` и тест. Второй тест проходит через строку, но ничего про неё не проверяет. Итог: первый тест `guarding`, второй `covered`.
- [ ] На 2 реальных репозиториях: доля защитных строк хоть с одним тестом; доля связей `history`, подтверждённых как `guarding`; медиана времени одной проверки `guarding`.

### E13. Проверка перед коммитом `why guard` (неделя 17)

**Цель:** предупредить в момент изменения, что правка задевает защитную строку.

**Шаги:**

1. Вход: `git diff --cached -U0` (по умолчанию), `--diff a..b` или unified diff из MCP `check_change`.
2. Каждую удалённую или изменённую строку сопоставить со строкой HEAD и её якорем.
3. Для задетых защитных строк сравнить канонические формы старого и нового оператора (E6). Изменение `FormatOnly` или `RenameOnly` — не нарушение.
4. Вывести: строку, задачу, от которой она защищает, прошлые поломки, связанные тесты.
5. С `--run-tests` запустить `guarding` и `history` тесты этих строк на рабочем дереве (раннер E12).
6. Код выхода 1, если задета защитная строка (`--fail-on touched`, по умолчанию) или если упал связанный тест (`--fail-on failing`).
7. `why guard --install-hook` пишет `.git/hooks/pre-commit`. Если хук уже есть — не перезаписывать, а вывести строку для ручного добавления (lefthook, husky).

**Ретроспективная оценка — главная метрика этапа (без агентов и без затрат на LLM):**

1. Для каждой цепочки A → B → C из эталона E11 состояние репозитория откатывается к родителю A.
2. На диф коммита A запускается `why guard` (история считается только до A).
3. Считается, предупредил бы инструмент в момент A: задевал ли A защитную строку более раннего багфикса.
4. Ложные тревоги: тот же прогон на 500 случайных коммитах, не связанных с регрессиями. Доля коммитов с предупреждением.

**Критерии приёмки:**

- [ ] На `fx-chain` `why guard` по дифу A выдаёт предупреждение и код 1.
- [ ] Полнота предупреждений на цепочках и доля ложных тревог записаны в `eval/results`.

### E14. Формат ответа, CLI и MCP-сервер (неделя 18)

**Цель:** ответ `swhy/1` по разделу «Интерфейсы» через CLI и MCP; три варианта формата для эксперимента N4.

**Шаги:**

1. `response.rs`: сборка `compact` и `full` из внутренних структур. Все словари — `BTreeMap`, порядок полей — порядок объявления в структурах. Никаких времён генерации и абсолютных путей.
2. Бюджет: оценка токенов = байты / 3,5. Порядок обрезки при превышении 1500: старые события не-багфиксы → `summary` задач → дифы (в `full`). Что обрезано — в `truncated`.
3. Snapshot-тесты `insta` на всех fixture: любое изменение ответа видно в ревью.
4. CLI на clap со всеми командами из «Интерфейсов». Формат `text` — человекочитаемый таймлайн, как в примере приложения.
5. MCP-сервер на rmcp: четыре инструмента, каждый — `Parameters<Req>` → `Json<Resp>`. Тяжёлая работа — в `spawn_blocking`. Ошибки — `isError: true` и короткое сообщение с кодом.
6. Описания инструментов — на английском, 2–4 предложения, фиксируются до эксперимента. Шаблон для `code_history`:

```
Refactoring-aware history of lines in a Swift or Objective-C file. Returns only meaningful
changes (formatting, moves and renames are skipped), linked Jira/GitHub tasks, protective
lines added by bug fixes, and the tests that guard them. Call it BEFORE you edit or delete
existing logic, especially checks that look redundant. Typical size: 300-1500 tokens.
```

7. Подключение к агентам — пример `.mcp.json` для Claude Code; то же для Codex и Cursor в `docs/`:

```json
{ "mcpServers": { "swhy": { "command": "swhy", "args": ["mcp"] } } }
```

8. Ответ держать далеко от лимитов Claude Code: предупреждение после 10 000 токенов вывода MCP, обрезка на 25 000 (`MAX_MCP_OUTPUT_TOKENS`).

**Критерии приёмки:**

- [ ] Интеграционный тест запускает `swhy mcp` как процесс, проходит `initialize` → `tools/list` → `tools/call` и сверяет `structuredContent` со snapshot.
- [ ] Проверка через `npx @modelcontextprotocol/inspector swhy mcp` без ошибок.
- [ ] На всех fixture ответ `compact` ≤ 1500 токенов; одинаковый запрос дважды даёт побайтово одинаковый ответ.
- [ ] Claude Code с `.mcp.json` видит сервер и вызывает `code_history` в ручной проверке.

### E15. Приложение на Tauri (неделя 19)

**Цель:** то же для людей: перетащил файл, выделил строки, увидел историю, задачи, защитные строки и тесты.

**Экраны и поведение:**

1. Пустое окно с зоной «Перетащите файл Swift или ObjC». Корень репозитория — через `gix::discover` от пути файла.
2. Слева — код файла в CodeMirror 6 только для чтения, подсветка Swift и ObjC из `@codemirror/legacy-modes`. Выделение строк мышью запускает запрос (с задержкой 300 мс после окончания выделения).
3. В левом поле у защитных строк — значок щита, при наведении — задача и причина.
4. Справа — таймлайн: события по дате, переезды, «скрыто N технических коммитов» (раскрывается по клику). У багфикса, вызвавшего регрессию, — пометка «вызвал IOS-…».
5. Под таймлайном — тесты защитных строк с уровнем и кнопкой «Запустить». Прогон идёт в фоне, прогресс приходит через `tauri::ipc::Channel`.
6. Ключ задачи — ссылка в браузер. Внизу — «Эксперты»: авторы багфиксов этих строк.

**Команды Rust:** `open_repo(path)`, `file_at(rev, path)`, `query(path, start, end, opts)`, `run_tests(ids)`. Все вызывают `swhy-core` и выполняются в `spawn_blocking`.

**Сборка:** `cargo tauri build --bundles app`. Подпись и нотаризация не нужны: для защиты достаточно неподписанного `.app`.

**Критерии приёмки:**

- [ ] Сценарий на `fx-chain`: перетащить файл, выделить строки, увидеть цепочку, щит на нужной строке и тест `guarding`.
- [ ] Тот же сценарий на одном реальном репозитории; скриншоты в `docs/screenshots/` для главы 3.

## Этапы E16–E17: бенчмарк и эксперименты

Эти этапы дают N4 и закрывают оценку N1–N6. Публичного бенчмарка агентов на Swift нет: в SWE-bench Multilingual Swift отсутствует, SWE-Bench Mobile закрыт и не компилирует код, в SwiftEval 28 маленьких задач на уровне функций. Набор задач собирается свой.

### E16. Бенчмарк задач-ловушек (неделя 20)

**Цель:** 15–25 задач, где «очевидное» исправление возвращает старый баг. Регрессия измеряется скрытым тестом.

**Источник 1 — реальные истории (основной).** Ищется в результатах E11 и эталоне: багфикс X внёс защитную строку; позже коммит D (исправление бага Y) сломал эту строку и вызвал баг Z; Z исправлен коммитом E с регрессионным тестом T\_Z.

1. Базовая ревизия задачи — родитель D.
2. Текст задачи — описание бага Y, переписанное без подсказок о решении.
3. `FAIL_TO_PASS` — тесты, добавленные в D для бага Y.
4. `PASS_TO_PASS` — все тесты на родителе D плюс **скрытый тест T\_Z**, перенесённый из E на базовую ревизию. На базе он зелёный (Z ещё нет). Он краснеет, если агент повторит ошибку D.
5. Если T\_Z не компилируется на базе — задача отбрасывается.

**Источник 2 — синтетические задачи** (если реальных меньше 15). Защитная строка с тестом уровня `guarding`. Пишется задача про соседний симптом, где самое простое исправление — убрать или упростить защитную строку. Такие задачи помечаются `synthetic` и считаются отдельно.

**Требования к задаче:**

- код собирается и тестируется через `swift test` в Docker на Linux, без симулятора: SPM-пакеты и модули приложений, которые собираются как пакеты;
- история репозитория сохраняется до базовой ревизии включительно, всё будущее удаляется: ветки и теги после базы, затем `git reflog expire --expire=now --all && git gc --prune=now`. Иначе агент найдёт будущий фикс через `git log --all` — так уже случалось на SWE-bench;
- описание в `bench/tasks/<id>/task.toml`:

```toml
id = "alamofire-0007"
repo = "https://github.com/Alamofire/Alamofire"
base = "<sha>"
source = "real"                  # real | synthetic
issue = """Текст задачи для агента"""
fail_to_pass = ["AlamofireTests/RequestTests/test_x()"]
pass_to_pass = "all"
hidden_guard = ["AlamofireTests/SessionTests/test_z()"]
hidden_patch = "hidden_guard.patch"  # добавляет T_Z на базу
timeout_min = 30
```

**Критерии приёмки:**

- [ ] Не меньше 15 задач, из них не меньше 8 реальных.
- [ ] Для каждой: на базе `FAIL_TO_PASS` красные, `PASS_TO_PASS` зелёные; эталонное решение из D делает `FAIL_TO_PASS` зелёными и роняет `hidden_guard`; решение из E (если есть) проходит всё.

### E17. Эксперименты (недели 21–22)

**Эксперимент 1 — агент (главный).** Каждая задача решается в четырёх вариантах:

| Вариант | Что доступно агенту |
| --- | --- |
| А. Без истории | Bash без `git log`, `git blame`, `git show`; MCP нет |
| Б. Обычный git | Bash с git; MCP нет |
| В1. Инструмент, `compact` | Bash с git + MCP-сервер, формат без дифов |
| В2. Инструмент, `full` | Bash с git + MCP-сервер, формат с дифами |

Сравнение В1 и В2 с Б отвечает на вопрос N4: какой минимум информации нужен агенту.

**Запуск одного прогона** (скрипт `eval/agent/run.py`, Python допустим для оценки):

```bash
claude --bare -p "$(cat prompt.md)" \
  --output-format stream-json --verbose \
  --model <полный id модели, один на весь эксперимент> \
  --max-turns 60 --max-budget-usd 3 \
  --permission-mode dontAsk --no-session-persistence \
  --mcp-config swhy-mcp.json --strict-mcp-config \   # только в В1, В2
  --disallowedTools "mcp__*" \                          # только в А, Б
  --disallowedTools "Bash(git log:*)" "Bash(git blame:*)" "Bash(git show:*)"  # только в А
```

- `--bare` обязателен: без него в прогон попадают CLAUDE.md, хуки, плагины и `.mcp.json` машины.
- Версия Claude Code закреплена (`npm i -g @anthropic-ai/claude-code@<версия>`, `DISABLE_AUTOUPDATER=1`). Каждый прогон — новый контейнер, пользователь не root.
- В вариантах В1 и В2 первое событие `system/init` проверяется: сервер в `mcp_servers` загружен, `mcp_server_errors` пуст. Иначе прогон бракуется и повторяется.
- Описания MCP-инструментов и формат ответа фиксируются до начала эксперимента и не меняются: любое изменение определений инструментов сбрасывает кэш промптов.
- Бюджет примерно: 20 задач × 4 варианта × 5 прогонов = 400 прогонов. Стоимость прогона заранее оценить на 5 пилотных; если не влезает в бюджет — уменьшить число вариантов до А, Б, В1.

**Что пишется из результата прогона:** `total_cost_usd`, `num_turns`, `duration_ms`, `modelUsage` по моделям (в `usage` нет токенов субагентов), токены `input`, `output`, `cache_read_input_tokens`, `cache_creation_input_tokens`, число вызовов каждого инструмента из `stream-json`, итоговый диф.

**Проверка после прогона:** в чистом контейнере применить диф, запустить `FAIL_TO_PASS`, `PASS_TO_PASS`, `hidden_guard`.

**Метрики:**

| Метрика | Формула |
| --- | --- |
| Решено | все `FAIL_TO_PASS` и все `PASS_TO_PASS` зелёные |
| Регрессия | хоть один `PASS_TO_PASS` красный |
| Старый баг вернулся | `hidden_guard` красный |
| **Стоимость решённой задачи без регрессий** (главная) | сумма стоимости всех прогонов варианта, включая неудачные / число прогонов «решено и `hidden_guard` зелёный» |
| Токены на задачу | по `modelUsage`, отдельно кэш-чтение и кэш-запись |
| Ходы и вызовы инструментов | `num_turns`, число вызовов git и MCP |

Сокращение токенов не равно сокращению стоимости: запись и чтение кэша могут давать до 80% счёта. Поэтому главная метрика — стоимость, а токены и ходы — объясняющие.

**Статистика:**

- минимум 5 прогонов на задачу и вариант, порядок случайный; разброс одинаковых прогонов 2–6 п. п. даже при температуре 0;
- среднее по прогонам внутри задачи, затем парное сравнение вариантов по задачам — критерий Уилкоксона;
- 95% доверительные интервалы — парный бутстрап по задачам, 10 000 выборок;
- размер эффекта — A12 Варга–Делани (пороги 0,56 / 0,64 / 0,71) или δ Клиффа.

**Эксперимент 2 — точность.** Сводная таблица метрик E4–E13 на эталонах с базовыми линиями (`git log -L`, `git blame -w -M -C`, B-SZZ, только ключевые слова).

**Эксперимент 3 — производительность.** На трёх самых больших репозиториях: задержка первого и повторного запроса (медиана, p90), размер кеша, доля попаданий в кеш разборов.

**Эксперимент 4 — эмпирика iOS (N6).** Доля методов, у которых `git log -L` теряет историю на переездах; доля багфиксов, внёсших новый баг.

**Эксперимент 5 — люди (пилот).** 5–10 разработчиков, план «внутри субъектов» со сменой порядка: два набора по 5 вопросов «почему так написано», «с какой версии так работает», «от чего защищает строка»; один набор через git и Xcode, другой через приложение. Время и правильность ответов, затем опросник SUS (средний балл по индустрии — 68). В тексте подавать как пилот: интервалы при таком n широкие.

**Критерии приёмки:**

- [ ] `eval/results/` содержит сырые результаты всех прогонов и скрипт, который строит из них все таблицы и графики главы 4.
- [ ] Для главной метрики посчитаны значение, 95% интервал и p-value для пар В1–Б, В1–А, В1–В2.

## Риски и запасные варианты

Каждый риск проверяется в конкретной точке плана. Сработал признак — агент сразу переходит на запасной вариант и пишет об этом в `docs/decisions.md`.

| Риск | Признак и где проверить | Запасной вариант |
| --- | --- | --- |
| tree-sitter-swift плохо разбирает реальный код | E2: доля файлов без `ERROR` < 98% на репозиториях E1 | Вспомогательная утилита на SwiftSyntax (`swhy-swiftsyntax`, Swift CLI), которую ядро вызывает как процесс и получает JSON символов. Только для файлов с ошибками |
| ObjC → Swift сопоставляется плохо | E9: точность на парах < 0,6 | Оставить метод → метод с `confidence: low` и показом кандидатов; соответствие строк ObjC → Swift — в «дальнейшее развитие» |
| Мало реальных задач-ловушек | E16: меньше 8 реальных | Добрать синтетическими, считать отдельно; расширить поиск на SPM-пакеты (Alamofire, Kingfisher, swift-\*) |
| Покрытие Xcode 26 ненадёжно, ID тестов Swift Testing не принимаются | E12, шаг 0 | Уровень `covered` и `guarding` считать через SPM-пакеты и `swift test`; для приложений — только `history` |
| Бюджет эксперимента с агентом | E17: пилот 5 прогонов дороже плана | Варианты А, Б, В1 без В2; дешевле модель, но одна на весь эксперимент; 5 прогонов вместо 9 |
| Нет Jira в открытых репозиториях | E1 | Оценка на GitHub Issues и PR; Jira — тесты на записанных ответах и демонстрация на рабочем проекте только с разрешения. Корпоративный код и данные в текст и репозиторий не попадают |
| Разметка эталонов отнимает слишком много времени | E1, E9, E11: отставание больше недели | Сократить: 50 методов для истории, 20 для строк, 100 пар для SZZ. Второй разметчик проверяет 30% выборки, считается каппа Коэна |
| Медленно на больших репозиториях (в gix нет Bloom-фильтров) | E3, E5: первый запрос > 10 с | Быстрый путь через `git log --full-history -- <path>`, предварительный `git commit-graph write --changed-paths` командой `swhy cache warm` |
| Появляются похожие инструменты | весь проект | Раз в месяц проверять `why` (quangdang46), Codebase Archaeology MCP, `sem-core`; формулировки новизны — только из раздела «Цель, границы и новизна» |
| Не хватает времени | контрольные точки календарного плана | Порядок урезания: 1) приложение → только CLI и MCP; 2) `guarding` — только для задач бенчмарка; 3) ObjC → Swift — замер и кандидаты без точности; 4) эксперимент с людьми — короткий опрос. Не урезается: N1–N4 и эксперимент с агентом |

## Источники

**Отслеживание истории кода**

- [CodeShovel, ICSE 2021](https://www.cs.ubc.ca/~rtholmes/papers/icse_2021_grund.pdf) — фазы сопоставления и пороги
- [CodeTracker, слайды FSE 2022](https://speakerdeck.com/tsantalis/accurate-method-and-variable-tracking-in-commit-history)
- [CodeTracker 2.0: отслеживание блоков](https://arxiv.org/html/2409.16185v1)
- [HistoryFinder](https://arxiv.org/html/2507.14716v1) — алгоритм, эталон, точность `git log -L`
- [RefactoringMiner](https://github.com/tsantalis/RefactoringMiner) и [статья об AST-дифе](https://arxiv.org/html/2403.05939v2)
- [GumTree, ASE 2014](https://www.labri.fr/perso/xblanc/data/papers/ASE14.pdf), [GumTree, ICSE 2024](https://www.labri.fr/perso/falleri/img/slides/icse24.pdf)
- [LHDiff, ICSM 2013](https://www.cs.usask.ca/~croy/papers/2013/LHDiffFullPaper-preprint.pdf)
- [Лимиты переименований git](https://code.googlesource.com/git/+/94b82d56866793018a7a9bcbe20c1c061fa41aa8%5E%21/)
- [Baresi et al., эволюция iOS-технологий, MOBILESoft 2024](https://air.unimi.it/handle/2434/1227037)

**SZZ, багфиксы, задачи**

- [Śliwerski et al., SZZ, MSR 2005](https://www.st.cs.uni-saarland.de/papers/msr2005/msr2005.pdf)
- [RA-SZZ](https://repositorio.ufrn.br/items/f6c7dec6-1194-4717-9016-8edcbdbca464/full)
- [Yin et al., How Do Fixes Become Bugs?, FSE 2011](https://www.eecg.utoronto.ca/~yuan/papers/incorrect_fix_abstract.html)
- [Jira Cloud REST API v3: issue](https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issues/)
- [GitHub REST: PR по коммиту](https://docs.github.com/en/rest/commits/commits#list-pull-requests-associated-with-a-commit)

**Тесты и мутации**

- [Just et al., мутанты и реальные ошибки, FSE 2014](https://homes.cs.washington.edu/~rjust/publ/mutants_real_faults_fse_2014.pdf)
- [Muter — мутационное тестирование Swift](https://github.com/muter-mutation-testing/muter)

**Стек**

- [tree-sitter 0.27](https://crates.io/crates/tree-sitter), [tree-sitter-swift 0.7.4](https://crates.io/crates/tree-sitter-swift), [tree-sitter-objc 3.0.2](https://crates.io/crates/tree-sitter-objc)
- [gix 0.90](https://crates.io/crates/gix), [статус возможностей gitoxide](https://raw.githubusercontent.com/GitoxideLabs/gitoxide/main/crate-status.md)
- [imara-diff 0.2](https://crates.io/crates/imara-diff), [rmcp 3.9](https://crates.io/crates/rmcp), [tauri 2.12](https://crates.io/crates/tauri)
- [Sourcegraph: переход с Monaco на CodeMirror](https://webflow.sourcegraph.com/blog/migrating-monaco-codemirror)
- [SE-0005: перевод имён ObjC в Swift](https://github.com/apple/Swift-evolution/blob/master/proposals/0005-objective-c-name-translation.md), [StringExtras.cpp](https://github.com/swiftlang/swift/blob/main/lib/Basic/StringExtras.cpp), [ImportName.cpp](https://github.com/swiftlang/swift/blob/main/lib/ClangImporter/ImportName.cpp)

**Агенты и аналоги**

- [HAFixAgent](https://arxiv.org/html/2511.01047v3)
- [SWE-Pruner](https://arxiv.org/abs/2601.16746)
- [Token Reduction Is Not Cost Reduction](https://arxiv.org/abs/2607.12161)
- [Codebase Archaeology MCP](https://glama.ai/mcp/servers/mgb3n4mmnz)
- [GitKraken MCP: инструменты](https://help.gitkraken.com/mcp/mcp-tools-reference)
- [mcp-server-git](https://pypi.org/project/mcp_server_git)
