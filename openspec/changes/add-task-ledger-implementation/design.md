# Design

## Context

The repo is the freshly bootstrapped rust-cli template: `src/main.rs` (clap derive, demo `hello`/`process`/`version`), `src/lib.rs` (demo functions + unit tests), `tests/integration_test.rs` (assert_cmd), edition 2021, toolchain rustc 1.99. Only `clap` is a dependency. There are no existing specs or persisted ledger data. Motivation and the storage model (status-only, frontmatter in `task.md`) are in proposal.md; behavior is in `specs/`.

## Goals / Non-Goals

**Goals:**
- Core library separate from the CLI layer so a future MCP adapter or cache can reuse it.
- Byte-preserving edits of `task.md` without a comment-preserving YAML library.
- Deterministic, hermetic integration tests (fixed date, temp roots).

**Non-Goals:**
- Configurable status sets or transition tables (fixed in code for v1).
- Performance work (no cache, no parallel scan); expected scale is hundreds of tasks.
- Windows-specific path handling beyond what std gives.

## Decisions

### Module layout: `core` vs `cli`
```
src/
  lib.rs            pub mod core; pub mod cli;
  core/
    mod.rs
    root.rs         find root, load ledger.toml (Config)
    record.rs       split frontmatter, typed parse, TaskView (output type)
    scan.rs         discover tasks + orphans
    resolve.rs      <task> reference resolution
    lifecycle.rs    Status enum, transition table
    edit.rs         surgical line edits + atomic write
    scaffold.rs     templates, built-ins, render
    check.rs        problem detection + fixes
    error.rs        LedgerError (thiserror) -> exit code + JSON code
  cli/
    mod.rs          clap types, run() -> ExitCode
    output.rs       human table, JSON envelopes, error printing
  main.rs           fn main() -> ExitCode { task_ledger::cli::run() }
  bin/ldg.rs        same one-liner
```
`core` never prints and never exits; it returns `Result<T, LedgerError>`. `cli` maps errors to stderr/JSON and exit codes. Two `[[bin]]` targets (`ledger` → `src/main.rs`, `ldg` → `src/bin/ldg.rs`) avoid Cargo's "file in multiple targets" warning. clap `name = "ledger"` so help reads the same from both.
Alternative: one binary + symlink install step — rejected, `cargo install` would not create it.

### YAML reading: `serde-saphyr` → `serde_json::Value`

> Implementation note: `serde_norway` had no release since 2024-12, so the fallback `serde-saphyr` (1.3, 2026-09) is used.
Frontmatter is split by hand (first line `---`, up to next line equal to `---`). The YAML text is parsed with `serde_norway` into `serde_json::Value`-compatible data (via `serde_norway::from_str::<serde_json::Value>`), then validated into a typed `Record` for known keys while the full map is kept for output (unknown keys exposed per cli-output-contract). Dates are kept as strings and validated with `jiff::civil::Date::from_str`.
Alternatives: `serde_yaml` (deprecated), `serde_yml` (RUSTSEC-2025-0068), `serde-saphyr` (fine, newer; keep as fallback if `serde_norway` proves inactive at implementation time — task 1.2 checks).

### Writes: line-level edits, never re-serialize
`edit.rs` operates on the frontmatter as `Vec<&str>` lines:
- `set_scalar(key, value)`: find a top-level line matching `^key:\s` (column 0). Replace the value span, keeping any trailing `\s+#...` comment. If absent, insert `key: value` just before the closing `---`.
- `append_history(entry)`: find top-level `history:` line. If its value is `[]` or empty-with-no-items, rewrite to block form. Insert `  - {at: D, to: S}` (plus `, note: "<json-escaped>"`) after the last line belonging to the block (subsequent lines that are indented or start with `-`). If absent, append `history:` + entry before the closing `---`.
- After editing, re-parse the result; on failure return `LedgerError::Invalid` and do not write.
- Atomic write: `tempfile::NamedTempFile::new_in(task_dir)`, write, `sync_all`, `persist(task.md)`.
Values written are always simple scalars (status words, dates, IDs), so quoting is only needed for `note` and `title` (both written as JSON strings, which are valid YAML).
Alternative: npm `yaml` Document API via Bun — rejected with the language choice; full re-serialization with serde — rejected, loses comments/order.

### Dates and determinism
`today()` uses `jiff::Zoned::now().date()`. Env var `LEDGER_TODAY=YYYY-MM-DD` overrides it (undocumented in help, used by tests).

### Errors and exit codes
`LedgerError` variants: `NotFound`, `Ambiguous{candidates}`, `Invalid`, `IllegalTransition{allowed}`, `Conflict`, `Io`, `Internal`. `code()` → 3/3/4/4/5/1/1, `json_code()` → spec strings. clap usage errors keep clap's exit 2 (`Cli::try_parse` then `e.exit()`). `check` with remaining problems returns exit 4 without being an error object.
`anyhow` is not used; a single error enum is enough.

### JSON output and schema
Output structs (`ListEnvelope`, `TaskEnvelope`, `CheckEnvelope`, `ErrorEnvelope`, `TaskOut`) derive `Serialize` + `schemars::JsonSchema`. `TaskOut` has typed known fields plus `#[serde(flatten)] extra: Map<String, Value>` for unknown keys. `ledger schema` emits a root schema with these as `$defs` via `schemars::schema_for!` on a wrapper enum. Integration test validates `ls --json` output against it using `jsonschema` (dev-dependency).

### Status as clap `ValueEnum`
`Status` derives `ValueEnum`, so an unknown status argument is a clap usage error (exit 2) listing valid values, matching the lifecycle spec. Transition table is a `match` on `(from, to)`.

### Scanning
Plain `std::fs::read_dir` two levels deep, skipping names starting with `.` or `_`. No `walkdir`/`ignore` dependency needed for a fixed depth.

### Built-in templates
`include_str!` from `src/core/templates/*.md`. The `task.md` template renders all required keys plus `history` with one entry; placeholders replaced via `str::replace`. `new` writes into a temp dir inside the project dir and renames it to the final name, so a validation failure never leaves a partial task (spec: removes partial directory).

## Risks / Trade-offs

- [Line-based edits miss unusual YAML (multi-line `status`, anchors, flow maps for the whole frontmatter)] → Edits only target top-level single-line keys; if the target key is not on a single line, fail with `Invalid` instead of guessing. Re-parse after edit as a safety net.
- [`serde_norway` maintenance] → Isolated behind `record.rs::parse_yaml`; swap to `serde-saphyr` is a one-function change.
- [Date-prefixed slug suffix matching can be surprising] → Ambiguity always fails with candidate list; exact ID always wins.
- [`schemars` flatten of arbitrary map produces a permissive schema] → Acceptable: contract is additive by design.

## Migration Plan

No persisted ledger data exists. Removing demo commands is a breaking CLI change for a not-yet-released binary; no consumers. Rollback = revert the change commit.
