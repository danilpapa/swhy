# E0 — чек-лист требований

Источник: `docs/TZ.md`. Каждый пункт — с точной цитатой. Этапов-входов у E0 нет.

## Шаги этапа

| № | Цитата ТЗ | Что должно существовать |
| --- | --- | --- |
| S1.1 | «Создать workspace по структуре из раздела «Архитектура и стек».» | `Cargo.toml` (workspace), `rust-toolchain.toml`, `crates/swhy-core`, `crates/swhy-cli`, `crates/swhy-mcp`, `crates/swhy-eval`, `app/` (`src-tauri/`, `ui/`), `fixtures/`, `bench/tasks/`, `eval/`, `docs/` |
| S1.2 | «`├── Cargo.toml              # workspace, edition = "2024", rust-version = "1.89"`» | `edition = "2024"`, `rust-version = "1.89"` в `[workspace.package]` |
| S1.3 | «`│   │   ├── model.rs        # типы из раздела «Термины и модель данных»`»; «Все типы живут в крейте `why-core`, модуль `model`. Все они `Serialize + Deserialize` (serde) и `Eq + Hash`, где это возможно.» | `crates/swhy-core/src/model.rs` с типами из блока «Структуры (Rust)» |
| S1.4 | «`parse/          # swift.rs, objc.rs, objc_prepass.rs, symbols.rs, canon.rs, skeleton.rs`», «`git/            # repo.rs, walk.rs, diff.rs, blame.rs`», «`track/          # symbol.rs (E4), moves.rs (E5), lines.rs (E7), lhdiff.rs, crosslang.rs (E9)`», «`classify.rs`», «`tracker/        # mod.rs, jira.rs, github.rs`», «`protect.rs`», «`chains.rs`», «`tests/          # discover.rs, history.rs, coverage.rs, guarding.rs (E12)`», «`guard.rs`», «`response.rs`», «`cache.rs`», «`query.rs`» | файлы модулей в `crates/swhy-core/src/` |
| S1.5 | «`│   ├── swhy-cli/           # clap`», «`│   ├── swhy-mcp/           # rmcp, stdio`», «`│   └── swhy-eval/          # инструменты эталонов и метрик`» | три крейта с этими зависимостями |
| S1.6 | «`├── app/                    # Tauri: src-tauri/ (зависит от swhy-core) + ui/ (TypeScript, Vite, CodeMirror 6)`» | `app/src-tauri` зависит от `swhy-core`; `app/ui` — TypeScript, Vite, CodeMirror 6 |
| S1.7 | «`├── fixtures/               # скрипты fx-*.sh, которые создают маленькие git-репозитории`», «`├── bench/tasks/            # задачи E16`», «`├── eval/                   # скрипты и результаты метрик`», «`└── docs/                   # progress.md, decisions.md, oracle-guide.md`» | каталоги и файлы |
| S1.8 | «Лицензия MIT OR Apache-2.0, `THIRD_PARTY.md` для заимствованных списков слов из компилятора Swift.» | `LICENSE-MIT`, `LICENSE-APACHE`, `license = "MIT OR Apache-2.0"`, `THIRD_PARTY.md` |
| S2 | «CI (GitHub Actions, `macos-latest`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`.» | `.github/workflows/ci.yml`, `runs-on: macos-latest`, три команды дословно |
| S3.1 | «Фреймворк fixture-репозиториев: `fixtures/lib.sh` с функциями `fx_init`, `fx_commit <msg> <date>`, `fx_write <path> <<EOF`.» | `fixtures/lib.sh` с тремя функциями и этими сигнатурами |
| S3.2 | «Автор, почта и даты фиксированы (`GIT_AUTHOR_DATE`, `GIT_COMMITTER_DATE`) — SHA детерминированы.» | автор и почта заданы в `lib.sh`, даты — через обе переменные; тест на значения автора, почты, дат |
| S3.3 | «Rust-хелпер `fixtures::build("fx-rename") -> PathBuf` собирает репозиторий в `target/fixtures/` один раз за прогон тестов.» | крейт с именем библиотеки `fixtures`, функция `build(&str) -> PathBuf`, путь `target/fixtures/<name>`, одна сборка за прогон; fixture `fx-rename` |
| S4 | «Заготовки `docs/progress.md`, `docs/decisions.md`, `docs/oracle-guide.md`.» | три файла |
| S5.1 | «`swhy --version`…» | бинарь `swhy`, флаг `--version` |
| S5.2 | «…и `swhy mcp` с пустым списком инструментов.» | `swhy mcp` по stdio: `initialize` → `tools/list` → `{"tools": []}` |

## Критерии приёмки

| № | Цитата ТЗ | Проверка |
| --- | --- | --- |
| A1 | «CI зелёный» | workflow существует; три команды проходят локально. Запуск на GitHub требует `git push` — вне полномочий исполнителя |
| A2 | «`fixtures::build` дважды даёт одинаковые SHA.» | тест: два вызова `build` — одинаковые SHA; две независимые сборки с нуля — одинаковые SHA всех коммитов; SHA HEAD закреплён константой |

## Требования из других разделов, которые касаются E0

| № | Цитата ТЗ | Проверка |
| --- | --- | --- |
| C1 | «в коде везде `swhy`: бинарь `swhy`, крейты `swhy-*`, конфиг `.swhy.toml`, переменная `SWHY_TRACKER_TOKEN`, схема ответа `swhy/1`» | имена пакетов в `Cargo.toml` |
| C2 | «Rust stable (версия закреплена в `rust-toolchain.toml`), git 2.45+» | `rust-toolchain.toml` с конкретной версией |
| C3 | «`rmcp` \| 3.9.0 \| `server`, `macros`, `transport-io`, `schemars`; вместе с `schemars = "1"`, `tokio`»; «`clap` \| 4.6.7 \| `derive`»; «`serde`, `serde_json` \| 1.0.229, 1.0.154»; «`insta`, `wiremock` \| последние \| dev» | версии в `Cargo.lock` |
| C4 | «Переопределить `get_info`, иначе имя сервера будет `rmcp`. В stdout — только JSON-RPC, все логи — в stderr.» | тест: `serverInfo.name == "swhy"`, каждая строка stdout — JSON-RPC |
| C5 | «Поддерживает MCP 2026-07-28 и старое рукопожатие `initialize` 2025-11-25.» | тест рукопожатия `initialize` 2025-11-25 |
| C6 | «`why mcp` \| MCP-сервер по stdio» | подкоманда `mcp` |

## Постоянные правила

| № | Цитата ТЗ | Как соблюдается в E0 |
| --- | --- | --- |
| P1 | «Никакой фоновой индексации. Всё считается по запросу, результат кешируется.» | нет фоновых потоков и демонов |
| P2 | «Код пользователя никуда не отправляется. Запросы к Jira, GitHub и LLM — только если они включены в `.why.toml` или флагом.» | сетевого кода нет |
| P3 | «Формат ответа (раздел «Интерфейсы») меняется только вместе с JSON-схемой, версией `schema` и snapshot-тестами.» | формат ответа в E0 не реализуется |
| P4 | «Любое решение, которого нет в ТЗ, записывается в `docs/decisions.md` в формате ADR: контекст, решение, отвергнутые варианты.» | ADR в `docs/decisions.md` |
| P5 | «Каждая метрика считается скриптом из `eval/`, запускается одной командой и пишет результат в `eval/results/<этап>-<дата>.json`.» | `eval/e0.sh` → `eval/results/E0-<дата>.json` |
| P6 | «Детерминизм: одинаковый вход даёт побайтово одинаковый JSON (сортировка, без времени генерации в теле ответа).» | результат `eval/e0.sh` без времени в теле; повторный запуск даёт тот же файл |
| P7 | «Ошибка разбора одного файла или одной ревизии не роняет запрос» | в E0 разбора нет |
| P8 | «Прогнать `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`.» | три команды после каждого шага |
| P9 | «Обновить `docs/progress.md`: этап, что сделано, цифры метрик, известные ограничения.» | запись E0 |
| P10 | «Коммит с префиксом `E<n>:`» | `git log --oneline` |
