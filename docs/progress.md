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
