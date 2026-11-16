# Ход работы

По одной записи на этап: дата, что сделано, коммиты, цифры метрик (только из `eval/results/`), известные ограничения. Принят ли этап, решает проверка: `docs/reviews/E<n>.md`.

---

## E0. Каркас — 2026-11-14

**Ветка:** `dev`. `git push` не выполнялся.

**Что сделано:**

- Workspace Cargo (`edition = "2024"`, `rust-version = "1.89"`, resolver 3): `crates/swhy-core`, `crates/swhy-cli` (бинарь `swhy`), `crates/swhy-mcp`, `crates/swhy-eval`, `fixtures` (пакет `swhy-fixtures`, библиотека `fixtures`), `app/src-tauri` (пакет `swhy-app`). Каталоги `app/ui`, `bench/tasks`, `eval`, `docs`.
- `swhy-core`: модель данных в `model.rs`; остальные модули структуры ТЗ — пустые файлы с указанием этапа.
- Лицензия MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`), `THIRD_PARTY.md`.
- `rust-toolchain.toml`: Rust 1.94.1, `rustfmt`, `clippy`.
- CI: `.github/workflows/ci.yml`, `macos-latest`, три команды из ТЗ.
- Фреймворк fixture: `fixtures/lib.sh` (`fx_init`, `fx_commit`, `fx_write` и `fx_mv`, `fx_rm`, `fx_branch`, `fx_checkout`, `fx_merge`), fixture `fx-smoke` и `fx-rename`, хелпер `fixtures::build`, бинарь `swhy-fixtures`.
- `swhy --version` печатает `swhy 0.1.0`; `swhy mcp` — MCP-сервер по stdio, `serverInfo.name = "swhy"`, `tools/list` возвращает пустой список (рукопожатие `initialize` 2025-11-25 и запросы без состояния 2026-07-28).
- `swhy-eval fixture-determinism` и скрипт `eval/e0.sh`.
- `docs/decisions.md` (ADR-1 … ADR-7), `docs/oracle-guide.md` (заготовка), `docs/work/E0-checklist.md`.

**Коммиты:**

```
a84ab8f E0: track TZ, agent definitions and observer skill; ignore dataset clones
ca861d5 E0: workspace, licenses, toolchain pin, swhy-core skeleton with the data model
26b8524 E0: fixture framework (lib.sh, fx-smoke, fx-rename, fixtures::build)
ae1f8c5 E0: swhy --version and swhy mcp with an empty tool list
0677cbc E0: swhy-eval fixture-determinism and eval/e0.sh
0124beb E0: app skeleton (src-tauri crate on swhy-core, ui package)
fc814fb E0: CI workflow (macos-latest: fmt, clippy, test)
2d0d7dd E0: decisions (ADR-1..7), oracle guide stub, E0 checklist
dddd6b6 E0: result files do not make the eval worktree dirty
9ad85f4 E0: eval result for fixture determinism
```

и коммит с этой записью.

**Цифры** — `eval/results/E0-2026-11-14.json` (команда `eval/e0.sh`, `head_sha` `dddd6b6d842648a760576ec26c55b8cbc1790d46`, `worktree_dirty: false`):

| Поле | Значение |
| --- | --- |
| `params.builds` | 2 |
| `metrics.fixtures_total` | 2 |
| `metrics.fixtures_identical` | 2 |
| `metrics.all_identical` | true |
| `metrics.fixtures.fx-rename.head` | `917c1cb2dda0bdab223ff8077f935b2b56fb65cf` (7 коммитов) |
| `metrics.fixtures.fx-smoke.head` | `bad3f5edc9c7d08b03097303aa1f0b0e1fd39712` (6 коммитов) |

Повторный запуск `eval/e0.sh` на том же HEAD дал побайтово тот же файл.

**Локальные проверки на Rust 1.94.1, macOS, git 2.46.1:** `cargo fmt --check` — код выхода 0; `cargo clippy --all-targets -- -D warnings` — 0; `cargo test --workspace` — 0, 33 теста пройдено, 0 провалено, 0 пропущено.

**Известные ограничения:**

- CI на GitHub не запускался: для этого нужен `git push`. Критерий «CI зелёный» проверен только локальным запуском тех же трёх команд.
- Сборка на Rust 1.89 (нижняя граница `rust-version`) не запускалась; по `cargo metadata` ни один из 106 внешних пакетов `Cargo.lock` не объявляет `rust-version` выше 1.89.
- `app/ui` не устанавливался и не собирался: на машине нет Node и npm. `app/src-tauri` пока не зависит от `tauri` (ADR-4).
- `fx-rename` создан как пример из ТЗ; ожидаемые события для него задаёт E4 и может изменить сценарий — тогда меняются закреплённые в `fixtures/tests/framework.rs` SHA.
- Найдены расхождения таблицы зависимостей ТЗ с crates.io, которые затронут следующие этапы (ADR-5): `tree-sitter` 0.27.2 и `tauri` 2.12.5 требуют Rust 1.90; у `reqwest` 0.13.5 нет фичи `rustls-tls`.
- В `data/repos/Signal-iOS` на момент E0 лежал незавершённый клон (`git log` сообщает «your current branch appears to be broken»). Каталог не трогался; это вход E1.

## E1. Датасет, замер проблемы и эталон — 2026-11-16

**Ветка:** `feature/setting-up-dataset`. `git push` не выполнялся.

**Что сделано:**

- `eval/repos.toml` закрепляет Signal-iOS на `a690a26d2cfe1bc15f99d73d2ef2054d41ba5e45` (HEAD клона пользователя в `data/repos/Signal-iOS`; клон полный, только читается).
- Замер проблемы: `swhy-eval measure`, скрипты `eval/measure/run.sh` (все четыре пункта → `eval/results/E1-measure.json`) и `eval/measure/1-repo-stats.sh` … `4-bugfix-share.sh` (один пункт в stdout). Fixture `fx-measure`, тесты с числами, выведенными из сценария fixture.
- Лексический сканер методов Swift и ObjC (`crates/swhy-eval/src/scan.rs`) — разбор появится в E2.
- Инструменты эталона: `eval/oracle/pool.sh`, `select.sh`, `sheet.sh`, `build.sh`, `check.sh` (кандидаты `git log -L` ∪ `git log --follow` ∪ merge-коммиты, трасса метода через переименования, переезды и переписывание ObjC → Swift, сборка файлов из меток, проверка состава, каппа Коэна).
- Правила разметки и формат файлов — `docs/oracle-guide.md`.
- Отбор эталона: выборка 1500 методов, трассы 154 кандидатов; из первых 50 годных один отклонён после проверки и заменён следующим (`eval/oracle/rejected.tsv`). Повторный запуск `eval/oracle/select.sh` даёт те же 50 методов и то же разбиение, `eval/oracle/build.sh` — побайтово те же файлы.
- Эталон: 50 методов Signal-iOS в `eval/oracle/methods/signal-ios-01.json` … `-50.json`, метки первого разметчика — `eval/oracle/labels/signal-ios.tsv`.
- `docs/decisions.md`: ADR-8 … ADR-13. `docs/work/E1-checklist.md`.

**Коммиты:**

```
1b2c881 E1: checklist of stage requirements
7b553bd E1: fixture fx-measure for the problem measurement
08f51c4 E1: problem measurement (items 1-4), eval/measure scripts, pinned Signal-iOS SHA
62a4a98 E1: oracle tooling (candidates, trace, labels, check, kappa) and annotation rules
d4c5edb E1: decisions ADR-8..13
782d717 E1: oracle selection (screening by trace, strata quotas, labels skeleton)
7f4c403 E1: oracle selection script, rejected list, selection rules in the guide
cb11ebf E1: oracle labels for methods 01-22 (07 rejected), class rules aligned with E6 definitions
babf462 E1: method-history oracle for Signal-iOS: 50 labeled methods, first annotator
9b47173 E1: test of the committed oracle against the stage requirements
3607928 E1: ADR-11 addendum (class boundaries, replaced method)
2339180 E1: measurement result for Signal-iOS (eval/results/E1-measure.json)
b6ab04d E1: oracle check result (eval/results/E1-2026-11-16.json)
```

и коммит с этой записью.

**Цифры замера** — `eval/results/E1-measure.json` (команда `eval/measure/run.sh`, `head_sha` `360792858d1b25786a0c632307bc55238e6ecd5c`, `worktree_dirty: false`, зерно 20261115, git 2.46.1):

| Пункт | Поле | Значение |
| --- | --- | --- |
| 1 | `repo_stats.commits` (из них merge) | 41004 (6017) |
| 1 | годы | 2014–2026 (первый коммит 2014-05-06, последний 2026-11-11) |
| 1 | `swift_files` | 2720 |
| 1 | `objc_files` | 75 (`.h` 41, `.m` 34, `.mm` 0) |
| 1 | `huge_commits` (больше 1000 файлов) | 4: `d267ec83` (2025-12-30, 2056 файлов, «Run `swiftformat .`»); `e10faa85` (2024-03-27, 1882 файлов, «Remove src/ prefix from SignalServiceKit files»); `52f0015d` (2023-06-22, 1847 файлов, «New set of icons.»); `370ff654` (2022-10-13, 1843 файлов, «Change license to AGPL») |
| 2 | выборка | 500 строк из 814922 в 2720 файлах; пустых в выборке 79 |
| 2 | `blame_w.technical_share` | 0.074 (37 строк: только пробелы 0, массовый коммит 37) |
| 2 | `blame_w_m_c_c.technical_share` | 0.074 (37 строк: только пробелы 0, массовый коммит 37) |
| 2 | `modes_differ_share` (режимы называют разные коммиты) | 0.17 (85 строк) |
| 3 | выборка | 200 методов из 24447; `log_failed` 0 |
| 3 | `ends_at_file_add_share` | 0.64 (128 методов) |
| 3 | `break_share` (обрыв на переезде) | 0.06 (12 методов: с ObjC-файлом 7, только Swift 5) |
| 3 | `median_commits` | 4 |
| 4 | `bugfix_share` (сообщение целиком) | 0.1328 (4647 из 34987 коммитов без merge) |
| 4 | `bugfix_by_subject_share` (первая строка) | 0.1217 (4259) |

**Цифры эталона** — `eval/results/E1-2026-11-16.json` (команда `eval/oracle/check.sh`, `head_sha` `2339180da2af8e9db37d046062eb7bc17683946d`, `worktree_dirty: false`):

| Поле | Значение | Требование ТЗ |
| --- | --- | --- |
| `methods` | 50 | 50 |
| `train` / `test` | 25 / 25 | 25 / 25 |
| `min_changing_commits` (без коммита появления) | 3 | не меньше 3 |
| `with_module_move` | 19 | не меньше 10 |
| `with_objc_origin` | 12 | не меньше 10 |
| `with_both` | 7 | — |
| `changing_commits` (с коммитами появления) | 374 | — |
| `candidates_union` (проверено кандидатов) | 9152 | — |
| `second_annotator.methods` | 0 из 15 | 30% методов |
| каппа Коэна | не посчитана | посчитана |

Классы коммитов: `FormatOnly` — 36, `Introduced` — 50, `MoveOnly` — 63, `Refactoring` — 12, `RenameOnly` — 5, `Semantic` — 204, `SignatureChanged: attributes` — 4. Переезды: `cross_language` — 12, `file` — 46, `module` — 26.

`eval/oracle/check.sh` завершается с кодом 1, в `metrics.violations`: «second annotator: 15 methods required, found 0»; «Cohen's kappa is not computed».

**Локальные проверки на Rust 1.94.1, macOS, git 2.46.1:** `cargo fmt --check` — код выхода 0; `cargo clippy --all-targets -- -D warnings` — 0; `cargo test --workspace` — 0, 73 теста пройдено, 0 провалено, 0 пропущено.

**Не сделано:**

- **Второй разметчик и каппа Коэна** (критерий «каппа посчитана»). Первый разметчик — исполнитель (агент); независимого второго разметчика у него нет (ADR-12). Подготовлены список 15 методов и порядок работы (`eval/oracle/second/README.md`), подсчёт каппы реализован и покрыт тестами.
- **Построчный эталон** (пункт 8) — по ТЗ размечается на неделе 10; в `docs/oracle-guide.md` записаны только правила.

**Известные ограничения:**

- Разметка сделана языковой моделью по листам проверки, а не человеком; это записано в поле `annotator` каждого файла. Проверочная часть (`test`) при разметке прочитана исполнителем этого этапа — иначе её нельзя разметить; для подбора порогов она не использовалась (подбора в E1 нет).
- Под `git blame -w` коммит, изменивший в файле только пробелы, не может быть автором строки, поэтому слагаемое «только пробелы» в пункте 2 равно 0 по построению; технические строки в этом замере дают только массовые коммиты (ADR-9).
- Пункт 3: `git log -L :<func>:<file>` без своего `xfuncname` для Swift не работает вовсе («no match» на методе с отступом), замер идёт с `diff.swift.xfuncname` из `params` (ADR-9). «Метод того же имени» сравнивается по имени без меток, поэтому тёзки (`viewDidLoad`) в удалённом файле тоже считаются обрывом.
- Пункт 4 — только ключевые слова из списка E8; из 4647 совпадений 4244 дают слова на `fix`.
- Полный прогон `eval/measure/run.sh` занял 32 минуты на 8 ядрах (40 минут, когда параллельно шли другие задачи): почти всё время — `git blame -w -M -C -C` по одному сгенерированному файлу `SignalServiceKit/Protos/Backups/Backup.pb.swift` (15 027 строк).
- В выборку пункта 2 попадают строки сгенерированных файлов (protobuf): ТЗ их не исключает.
- Эталон смещён отбором: методы не длиннее 80 строк, от 3 до 12 изменяющих коммитов. Тесты и сгенерированный код исключаются только по пути: в эталон попали метод из `SignalServiceKit/Mocks/` (`signal-ios-26`) и метод из `TSInteraction+SDS.swift` (`signal-ios-43`), происхождение этого файла (ручной или сгенерированный) не проверялось.
- Ключи методов построены лексическим сканером E1, а не разбором `swhy-core`; после E2 их нужно сверить. Сканер не различает две реализации метода под `#if`.
- Источник кандидатов «текущая версия инструмента» пуст (`candidates.tool = 0`): истории в `swhy` ещё нет.
- Долю коммитов со ссылкой на задачу E1 не измерял. ТЗ отмечает, что у Signal-iOS «почти нет ссылок на задачи» (риск «Нет Jira в открытых репозиториях»); отчёт об этой доле — критерий E8.
