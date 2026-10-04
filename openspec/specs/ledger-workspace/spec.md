# ledger-workspace Specification

## Purpose

Defines how the CLI locates a ledger root, which folders count as tasks, and how a user-supplied task reference resolves to exactly one task.

## Requirements

### Requirement: Root discovery
The CLI SHALL locate the ledger root by walking up from the current directory to the first directory containing `ledger.toml`. When `LEDGER_ROOT` is set, the CLI SHALL use that directory instead and SHALL require it to contain `ledger.toml`.

#### Scenario: Root found from a nested directory
- **WHEN** the user runs `ledger ls` inside `<root>/ingest/2026-09-12-cdc-backfill/`
- **THEN** the CLI uses `<root>` as the ledger root

#### Scenario: Environment override
- **WHEN** `LEDGER_ROOT=/vault` is set and `/vault/ledger.toml` exists
- **THEN** the CLI uses `/vault` regardless of the current directory

#### Scenario: No root
- **WHEN** no `ledger.toml` exists in the current directory or any ancestor and `LEDGER_ROOT` is unset
- **THEN** the CLI exits with code 3 and reports that no ledger root was found

### Requirement: Configuration file
`ledger.toml` SHALL be valid TOML. An empty file SHALL be valid. The optional key `templates_dir` (default `_templates`) SHALL name the templates directory relative to the root. Unknown keys SHALL be ignored.

#### Scenario: Empty config
- **WHEN** `ledger.toml` is empty
- **THEN** the CLI runs with defaults

#### Scenario: Invalid TOML
- **WHEN** `ledger.toml` is not valid TOML
- **THEN** the CLI exits with code 4 and names the file and parse error

### Requirement: Task layout
A task SHALL be a directory at exactly `<root>/<project>/<slug>/`. Directories whose name starts with `.` or `_` SHALL be ignored at both levels. The task ID SHALL be `<project>/<slug>`. A task directory is a ledger task only when it contains `task.md`.

#### Scenario: Ignored directories
- **WHEN** the root contains `_templates/default/task.md` and `.git/`
- **THEN** neither appears as a task

#### Scenario: Task identified by path
- **WHEN** `<root>/ingest/2026-09-12-cdc-backfill/task.md` exists
- **THEN** a task with ID `ingest/2026-09-12-cdc-backfill` is listed

### Requirement: Task reference resolution
Commands taking `<task>` SHALL resolve it, in order, as: (1) an exact task ID; (2) a filesystem path (absolute or relative to the current directory, e.g. `.`) pointing to a task directory; (3) a slug suffix — a task whose slug equals the reference or ends with `-<reference>`. If the first matching step yields more than one task, the CLI SHALL fail as ambiguous.

#### Scenario: Exact ID
- **WHEN** the user runs `ledger show ingest/2026-09-12-cdc-backfill`
- **THEN** that task is shown

#### Scenario: Current directory
- **WHEN** the user runs `ledger show .` inside a task directory
- **THEN** that task is shown

#### Scenario: Unique slug suffix
- **WHEN** the user runs `ledger show cdc-backfill` and only one slug ends with `-cdc-backfill`
- **THEN** that task is shown

#### Scenario: Ambiguous suffix
- **WHEN** two tasks in different projects have slugs ending with `-ci-cache`
- **THEN** `ledger show ci-cache` exits with code 3 and lists the candidate IDs

#### Scenario: Not found
- **WHEN** no step matches the reference
- **THEN** the CLI exits with code 3
