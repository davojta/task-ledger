# Proposal

## Why

Spec-driven work lives in `project/task/{input,proposal,design}.md` folders, but nothing tracks where each task is in its life. Existing tools either move folders to encode status (unstable paths, broken links) or keep a central ledger that drifts from the task files. A small CLI that owns a `status` field in a per-task `task.md` frontmatter gives one source of truth that Obsidian, nvim, git and agents all read natively, and gives agents a stable `--json` contract to drive it.

## What Changes

- **BREAKING**: Remove the template demo commands (`hello`, `process`, `version`) and their library functions and tests. Version stays available via `--version`.
- Ship the CLI as two binaries, `ledger` and the short alias `ldg`, with identical behavior.
- Discover the ledger root by walking up to `ledger.toml` (override: `LEDGER_ROOT`).
- Define the `task.md` record: YAML frontmatter (`schema`, `id`, `title`, `project`, `status`, `stage`, `priority`, `tags`, `created`, `updated`, `status_changed`, `history`) plus a free-text body the CLI never touches. Tasks never move; status is a field, not a folder.
- Add commands:
  - `ledger ls` — list/filter tasks; hides `done`/`dropped` unless `--all`.
  - `ledger show <task>` — one record plus artifact presence.
  - `ledger status <task> <status>` (alias `mv`) — validated lifecycle transition with history.
  - `ledger new <project>/<slug>` — scaffold a task folder from templates.
  - `ledger check` — validate records, report orphans, optional safe `--fix`.
  - `ledger schema` — print the JSON Schema of the `--json` output.
- Agent contract: never prompt, versioned JSON envelope, JSON errors, fixed exit codes, surgical atomic writes that preserve unknown keys, comments and body.
- Ship an agent instructions snippet (`docs/agents-snippet.md`) and update README/CLAUDE.md/Makefile.

Out of scope (deferred): `archive` command, `depends_on`, SQLite cache, MCP server, configurable status sets, Obsidian Bases view files.

## Capabilities

### New Capabilities
- `ledger-workspace`: root discovery via `ledger.toml`, task discovery by layout, and task reference resolution.
- `task-record`: the `task.md` frontmatter format, field semantics, derived stage, and the write-safety guarantees for CLI edits.
- `task-lifecycle`: the status set, allowed transitions, and the `status` command behavior including history.
- `task-query`: listing, filtering, sorting and showing tasks.
- `task-scaffolding`: creating task folders from templates with `new`.
- `ledger-validation`: the `check` command — record validation, orphan detection, safe fixes.
- `cli-output-contract`: binaries, non-interactive behavior, JSON envelopes, error format, exit codes, and `schema`.

### Modified Capabilities
<!-- none: no existing specs -->

## Impact

- Code: `src/main.rs` and `src/lib.rs` rewritten; new modules under `src/` (core vs cli split); new `src/bin/ldg.rs`; `tests/integration_test.rs` replaced.
- `Cargo.toml`: two `[[bin]]` targets; new dependencies (serde, serde_json, a maintained YAML reader, toml, jiff, thiserror, tempfile, schemars) and dev-dependency `jsonschema`.
- Docs: README, CLAUDE.md, Makefile `run-main`, new `docs/agents-snippet.md`.
- No persisted data exists yet; no migration needed.
