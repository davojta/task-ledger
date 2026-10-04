# Tasks

## 1. Scaffolding and dependencies

- [x] 1.1 Remove demo `hello`/`process`/`version` code from `src/main.rs`, `src/lib.rs` and `tests/integration_test.rs`; create the `core/` and `cli/` module skeleton from design.md; verify `cargo build` succeeds
- [x] 1.2 Check `serde_norway` release activity (fall back to `serde-saphyr` per design if inactive); add dependencies (serde, serde_json, serde_norway, toml, jiff, thiserror, tempfile, schemars) and dev-dependencies (tempfile, jsonschema); verify `cargo build` succeeds
- [x] 1.3 Add `[[bin]]` targets `ledger` (`src/main.rs`) and `ldg` (`src/bin/ldg.rs`), both calling `task_ledger::cli::run()`; verify `cargo run --bin ledger -- --version` and `cargo run --bin ldg -- --version` print the package version
- [x] 1.4 Add a shared test helper in `tests/common/mod.rs` that builds a temp ledger root (`ledger.toml`, tasks from inline fixtures) and runs a binary with `LEDGER_ROOT` and `LEDGER_TODAY` set; verify a smoke test using it passes

## 2. Errors and output contract

- [x] 2.1 Implement `LedgerError` with exit-code and JSON-code mapping, and `cli::run()` error handling (stderr text, `{"error":...}` on stdout with `--json`, clap exit 2); verify unit tests cover every variant's codes
- [x] 2.2 Implement output types (`TaskOut` with flattened extras, list/task/check/error envelopes) deriving `Serialize` + `JsonSchema`; verify a unit test serializes a `TaskOut` with an unknown key and `schema_version: 1`

## 3. Workspace

- [x] 3.1 Implement root discovery (walk-up to `ledger.toml`, `LEDGER_ROOT` override) and `Config` loading (`templates_dir`, unknown keys ignored); verify unit tests for nested dir, env override, missing root (exit 3), invalid TOML (exit 4)
- [x] 3.2 Implement task scan (`<project>/<slug>/task.md`, skip `.`/`_` names) and orphan discovery; verify unit tests with a temp tree including `_templates/` and `.git/`
- [x] 3.3 Implement `<task>` resolution (exact ID → path incl. `.` → slug suffix, ambiguity error with candidates); verify unit tests for each scenario in `specs/ledger-workspace/spec.md`

## 4. Task record

- [x] 4.1 Implement frontmatter split and YAML parse into full map + typed known fields with validation (required keys, status, dates, schema, priority, tags); verify unit tests for minimal valid record, missing frontmatter, each invalid value
- [x] 4.2 Implement path-derived `id`/`project` and derived `stage` (last non-empty of input/proposal/design; explicit wins) and artifact presence; verify unit tests for the task-record derived-stage scenarios
- [x] 4.3 Implement `edit.rs`: `set_scalar` (keeps trailing comments, inserts if missing, rejects multi-line values), `append_history` (block, `[]`, absent cases; JSON-quoted note), re-parse guard, atomic write via `NamedTempFile::persist`; verify unit tests assert byte-for-byte preservation of unknown keys, comments, blank lines and body

## 5. Query commands

- [x] 5.1 Implement `ledger ls` with filters (`--project`, `--status`, `--stage`, `--tag`, `--all`), sort (`id|updated|created|status`), outputs (table, `--json`, `--jsonl`, `--paths`), and skip-with-warning for broken records; verify integration tests for each task-query scenario
- [x] 5.2 Implement `ledger show <task>` with `--json` and `--body`; verify integration tests for artifact presence, not-found JSON error (exit 3) and broken record (exit 4)

## 6. Lifecycle

- [x] 6.1 Implement `Status` (`ValueEnum`, lifecycle order) and the transition table with `--reopen`/`--force`; verify unit tests enumerate allowed and illegal pairs
- [x] 6.2 Implement `ledger status <task> <status>` (alias `mv`) with `--note`, `--dry-run`, `--json`, idempotent no-op; verify integration tests for every task-lifecycle scenario, including unchanged bytes/mtime on no-op and dry-run and comment preservation on a real file

## 7. Scaffolding command

- [x] 7.1 Add built-in templates (`src/core/templates/{task,input,proposal,design}.md`) and the renderer (`{{id}}`, `{{title}}`, `{{project}}`, `{{date}}`, `{{status}}`); verify a unit test that the rendered built-in `task.md` passes record validation
- [x] 7.2 Implement `ledger new <project>/<slug>` (`--title`, `--status`, `--template`, `--no-date-prefix`, `--json`) with name validation, conflict check, temp-dir-then-rename creation and cleanup on invalid template; verify integration tests for every task-scaffolding scenario

## 8. Validation command

- [x] 8.1 Implement `ledger check` problem detection (all codes incl. `orphan`) with human and `--json` output and exit 4 on problems; verify integration tests for clean ledger, invalid status, orphan JSON
- [x] 8.2 Implement `--fix` for `id_mismatch`, `project_mismatch`, missing `schema` using `edit.rs`; verify integration tests that fixes are surgical and that unfixable problems keep exit 4

## 9. Schema command and docs

- [x] 9.1 Implement `ledger schema`; verify an integration test validates `ls --json`, `show --json`, `check --json` and an error envelope against it with `jsonschema`
- [x] 9.2 Write `docs/agents-snippet.md` (~15 lines: use `ledger ls --json`, change status only via `ledger status`, never move task folders, exit codes); verify every command in it runs against a fixture root
- [x] 9.3 Update README.md, CLAUDE.md and Makefile (`run-main` → `cargo run --bin ledger -- ls`) for the new commands, binaries and `ledger.toml`; verify the README usage commands run as written against a fixture root

## 10. Integration check

- [x] 10.1 Run `make check` and `make run-all-tests`; verify both pass with zero clippy warnings
- [x] 10.2 End-to-end manual run in a temp dir: `ledger.toml` → `new` → `status` through `review`/`done` → `ls --all --json` → `check`; verify `ldg` produces identical output and `task.md` diffs show only owned-key lines changed
