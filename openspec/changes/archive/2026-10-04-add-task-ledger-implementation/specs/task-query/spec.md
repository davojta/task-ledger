# Spec Delta

## Purpose

Lets users and agents list, filter, sort and inspect tasks without moving or opening task folders.

## ADDED Requirements

### Requirement: List tasks
`ledger ls` SHALL list all valid tasks under the root. By default it SHALL exclude tasks with status `done` or `dropped`; `--all` SHALL include them.

#### Scenario: Default hides terminal tasks
- **WHEN** the ledger has one `active` and one `done` task
- **THEN** `ledger ls` lists only the `active` task

#### Scenario: Include all
- **WHEN** the user runs `ledger ls --all`
- **THEN** both tasks are listed

### Requirement: Filters
`ledger ls` SHALL support `--project <name>`, `--status <s1,s2,...>`, `--stage <stage>` (matching the reported stage) and `--tag <tag>`. Filters SHALL combine with AND. An explicit `--status` SHALL override the default terminal-status exclusion.

#### Scenario: Status filter includes done
- **WHEN** the user runs `ledger ls --status done`
- **THEN** only `done` tasks are listed

#### Scenario: Combined filters
- **WHEN** the user runs `ledger ls --project ingest --tag task`
- **THEN** only tasks in project `ingest` whose `tags` contain `task` are listed

### Requirement: Sorting
`ledger ls` SHALL sort by task ID ascending by default. `--sort <id|updated|created|status>` SHALL change the key; `updated` and `created` SHALL sort newest first; `status` SHALL follow lifecycle order `idea, backlog, active, review, done, dropped`. Ties SHALL break by ID.

#### Scenario: Sort by updated
- **WHEN** the user runs `ledger ls --sort updated`
- **THEN** the most recently updated task appears first

### Requirement: Output modes
`ledger ls` SHALL print a human table (ID, STATUS, STAGE, UPDATED, TITLE) by default; `--json` SHALL print the list envelope; `--jsonl` SHALL print one task object per line with no envelope; `--paths` SHALL print one absolute task directory path per line.

#### Scenario: Paths output
- **WHEN** the user runs `ledger ls --paths`
- **THEN** each line is an absolute path to a task directory

### Requirement: Show task
`ledger show <task>` SHALL print one task's fields and artifact presence (`input`, `proposal`, `design`: present and non-empty). `--body` SHALL also print the `task.md` body.

#### Scenario: Show with artifacts
- **WHEN** a task has `input.md` and `proposal.md` but no `design.md`
- **THEN** `ledger show <task> --json` reports `artifacts` as `{input: true, proposal: true, design: false}`

### Requirement: Invalid records do not block listing
A task whose `task.md` fails to parse SHALL be skipped by `ls` with a warning on stderr naming its path; `ls` SHALL still exit 0. `show` on such a task SHALL exit with code 4.

#### Scenario: One broken record
- **WHEN** one of three tasks has malformed frontmatter
- **THEN** `ledger ls` lists the other two, warns about the third, and exits 0
