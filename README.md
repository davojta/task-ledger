# task-ledger

CLI to manage spec driven artefacts for the task. Bootstrapped from the [rust-cli template](https://github.com/davojta/project-templates/tree/main/rust-cli).

## What It Is

A status-only task ledger for spec-driven work. Tasks never move; status is a field in the `task.md` frontmatter. The file is readable by Obsidian, nvim, git and agents.

- Single source of truth: YAML frontmatter in `task.md`
- No central database drift: agents and humans edit the same `task.md`
- Stable paths: `<root>/<project>/<slug>/task.md` never changes
- Safe writes: CLI preserves unknown keys, comments, and body text

## Install

```bash
cargo install --path .
```

Installs two binaries: `ledger` and `ldg` (alias, identical behavior).

## Setup

Create a `ledger.toml` at the vault/repo root:

```toml
# Optional; relative to ledger root, defaults to _templates
templates_dir = "_templates"
```

Task layout:

```
<root>/
├── ledger.toml
├── <project>/
│   └── <slug>/
│       └── task.md        # source of truth
├── _templates/            # optional
│   └── default/
│       └── task.md        # template for `ledger new`
```

Templates: `ledger new --template <name>` copies every `.md` file from `_templates/<name>/` (must include `task.md`) and replaces `{{id}}`, `{{title}}`, `{{project}}`, `{{date}}`, `{{status}}`. Without `_templates/default/`, built-in `task.md`, `input.md`, `proposal.md` and `design.md` are used.

## Example: New Task

```bash
# Create a task in the ingest project
ledger new ingest/sync --title "Sync downstream data"   # -> ingest/<today>-sync

# Check it
ledger show sync

# Change status
ledger status sync active
ledger status sync review --note "Ready for validation"
ledger status sync done
```

## Task Lifecycle

```
idea ──► backlog ──► active ──► review ──► done
  │         │          │  ▲        │
  └─────────┴──────────┴──┴────────┴──► dropped
```

- `review → active` is allowed.
- `done`/`dropped → backlog` needs `--reopen`.
- `--force` allows any transition; setting the current status is a no-op.

## Command Reference

| Command | Purpose |
|---------|---------|
| `ledger ls [--all] [--project P] [--status S] [--sort id\|created\|updated\|status]` | List tasks (hides done/dropped unless `--all`) |
| `ledger show <task>` | Show task record and artifact presence |
| `ledger status <task> <status>` (alias `mv`) | Change task status with optional `--note "..."` |
| `ledger new <project>/<slug> [--title "..."]` | Create a task from template |
| `ledger check [--fix]` | Validate all task records, report orphans |
| `ledger schema` | Print JSON Schema of `--json` output |

All commands support `--json` for machine-readable output.

## Example Task File

```yaml
---
schema: 1
id: ingest/2026-10-04-sync
title: Sync downstream data
project: ingest
status: active
created: 2026-10-04
updated: 2026-10-04
status_changed: 2026-10-04
tags: [task]
history:
  - {at: 2026-10-04, to: backlog}
  - {at: 2026-10-04, to: active}
---

Populate the metrics cache with fresh CDC events.
Progress tracked in PR #123.
```

## JSON Contract & Exit Codes

See [docs/agents-snippet.md](docs/agents-snippet.md) for agent integration: JSON envelopes, exit codes (0 ok, 1 internal, 2 usage, 3 not found, 4 invalid, 5 conflict), and error format.

## Development

```bash
# Build
make build

# Run tests
make run-all-tests

# Lint and format checks
make check

# All make targets
make help
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
├── common/mod.rs        # temp ledger fixture
└── *_test.rs            # integration tests per command
```

## Dependencies

Runtime: clap (v4), serde, serde-saphyr, serde_json, schemars, jiff, thiserror, toml

Dev: assert_cmd, predicates, tempfile, jsonschema
