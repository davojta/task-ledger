# Project: task-ledger

Status-only task ledger CLI: spec-driven work tracked in `task.md` frontmatter. Tasks never move; status is a field. One source of truth readable by Obsidian, nvim, agents, and git.

## Tech Stack

- **Runtime**: Rust (stable, edition 2021)
- **Package Manager**: Cargo
- **CLI Framework**: clap (v4, derive)
- **Data**: serde, serde-saphyr (YAML), serde_json, schemars, jiff, thiserror, toml
- **Testing**: built-in `cargo test` + assert_cmd, predicates, tempfile, jsonschema
- **Lint**: clippy
- **Format**: rustfmt

## Commands

```bash
# Build and test
make build                # Build the project
make run-all-tests        # Unit + integration tests
make run-lint             # Lint with clippy
make run-format           # Format with rustfmt
make check                # Lint + format check (CI target)

# Direct cargo
cargo run --bin ledger -- ls                    # List tasks
cargo run --bin ledger -- show <task>           # Show task
cargo run --bin ledger -- status <task> active  # Change status
cargo run --bin ledger -- new <project>/<slug>  # Create task
cargo run --bin ledger -- check                 # Validate
cargo run --bin ledger -- schema                # Print JSON Schema
```

## Project Structure

```
src/
├── main.rs              # `ledger` binary
├── bin/ldg.rs           # `ldg` binary (same entry point)
├── lib.rs
├── cli/                 # clap definitions, dispatch, output; never touches files directly
│   ├── mod.rs           # commands, flags, error -> exit code mapping
│   ├── output.rs        # JSON envelopes and error output
│   └── ls.rs show.rs status.rs new.rs check.rs schema.rs
└── core/                # never prints or exits; returns Result<_, LedgerError>
    ├── root.rs          # ledger.toml discovery and config
    ├── scan.rs          # <project>/<slug> task and orphan discovery
    ├── resolve.rs       # <task> reference resolution
    ├── record.rs        # task.md parsing, validation, derived stage
    ├── edit.rs          # line-level frontmatter edits, atomic writes
    ├── lifecycle.rs     # statuses and transitions
    ├── scaffold.rs      # templates and task creation
    ├── check.rs         # validation and safe fixes
    ├── date.rs          # today (LEDGER_TODAY override)
    ├── error.rs         # LedgerError, exit and JSON codes
    └── templates/       # built-in task/input/proposal/design templates

tests/
├── common/mod.rs        # temp ledger fixture (LEDGER_ROOT, LEDGER_TODAY)
└── *_test.rs            # integration tests per command
```

Invariants: writes to `task.md` go through `core::edit` only (never re-serialize YAML); `--json` output types live in `cli/output.rs` and changing a field requires bumping `SCHEMA_VERSION`.

## Validation

```bash
# After each non-trivial edit
make run-all-tests && make run-lint

# Before committing
make check
```
