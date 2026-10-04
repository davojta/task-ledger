# Spec Delta

## Purpose

Creates new task folders with a valid `task.md` and starter artifacts from user-editable templates, so every task starts in a consistent shape.

## ADDED Requirements

### Requirement: Create task
`ledger new <project>/<slug>` SHALL create `<root>/<project>/<YYYY-MM-DD>-<slug>/` using today's date, unless `--no-date-prefix` is given. `<project>` and `<slug>` SHALL be non-empty, contain only lowercase letters, digits and `-`, and SHALL NOT start with `.` or `_`. The project directory SHALL be created if missing.

#### Scenario: New task with date prefix
- **WHEN** the user runs `ledger new ingest/cdc-backfill --title "CDC backfill"` on 2026-09-12
- **THEN** `ingest/2026-09-12-cdc-backfill/task.md` exists with `id: ingest/2026-09-12-cdc-backfill`, `title: CDC backfill`, `project: ingest`

#### Scenario: Invalid name
- **WHEN** the user runs `ledger new Ingest/My_Task`
- **THEN** the CLI exits with code 4 and creates nothing

#### Scenario: Already exists
- **WHEN** the target directory already exists
- **THEN** the CLI exits with code 5 and changes nothing

### Requirement: Initial record values
The new record SHALL have `schema: 1`, `status` from `--status` (default `backlog`, any valid status allowed), `created`, `updated` and `status_changed` set to today, and a `history` with one entry `{at: <today>, to: <status>}`. `title` SHALL default to the slug without date prefix.

#### Scenario: Custom initial status
- **WHEN** the user runs `ledger new ingest/x --status idea`
- **THEN** the record has `status: idea` and one history entry to `idea`

### Requirement: Templates
`--template <name>` (default `default`) SHALL select `<root>/<templates_dir>/<name>/`. Every `.md` file in it SHALL be copied into the new task with placeholders `{{id}}`, `{{title}}`, `{{project}}`, `{{date}}` and `{{status}}` replaced by plain string substitution. If the `default` template directory does not exist, the CLI SHALL use built-in templates for `task.md`, `input.md`, `proposal.md` and `design.md`. A missing non-default template SHALL exit with code 3.

#### Scenario: User template
- **WHEN** `_templates/default/input.md` contains `# {{title}}`
- **THEN** the new task's `input.md` contains `# CDC backfill`

#### Scenario: Built-in fallback
- **WHEN** no `_templates/default/` exists
- **THEN** the new task contains `task.md`, `input.md`, `proposal.md` and `design.md`

#### Scenario: Template result must be valid
- **WHEN** the rendered `task.md` fails record validation
- **THEN** the CLI exits with code 4 and removes the partially created task directory

### Requirement: New task output
On success the CLI SHALL print the created task (human summary, or the single-task envelope with `--json`).

#### Scenario: JSON output
- **WHEN** the user runs `ledger new ingest/x --json`
- **THEN** stdout is a single-task envelope whose `task.id` is the new ID
