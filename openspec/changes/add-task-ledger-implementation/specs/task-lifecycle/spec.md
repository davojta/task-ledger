# Spec Delta

## Purpose

Defines the fixed set of task statuses, which transitions between them are allowed, and how the `status` command records a transition.

## ADDED Requirements

### Requirement: Status set
`status` SHALL be one of `idea`, `backlog`, `active`, `review`, `done`, `dropped`. `done` and `dropped` are terminal.

#### Scenario: Unknown status argument
- **WHEN** the user runs `ledger status <task> started`
- **THEN** the CLI exits with code 2 and lists the valid statuses

### Requirement: Allowed transitions
Without `--force`, the CLI SHALL allow only: `idea→backlog`, `backlog→active`, `active→review`, `review→done`, `review→active`, any non-terminal status `→dropped`, and `done|dropped→backlog` only with `--reopen`. Any other transition SHALL exit with code 4 and leave the file unchanged. `--force` SHALL allow any transition.

#### Scenario: Allowed transition
- **WHEN** a task is `active` and the user runs `ledger status <task> review`
- **THEN** the status becomes `review` and the CLI exits 0

#### Scenario: Illegal transition
- **WHEN** a task is `idea` and the user runs `ledger status <task> done`
- **THEN** the CLI exits with code 4 and names the allowed targets

#### Scenario: Reopen requires flag
- **WHEN** a task is `done` and the user runs `ledger status <task> backlog` without `--reopen`
- **THEN** the CLI exits with code 4

#### Scenario: Reopen
- **WHEN** a task is `done` and the user runs `ledger status <task> backlog --reopen`
- **THEN** the status becomes `backlog`

#### Scenario: Forced transition
- **WHEN** a task is `idea` and the user runs `ledger status <task> done --force`
- **THEN** the status becomes `done`

### Requirement: Transition side effects
A successful status change SHALL set `status`, set `updated` and `status_changed` to today, and append `{at: <today>, to: <status>}` to `history`, including `note` when `--note` is given. Missing `status_changed` or `history` keys SHALL be added.

#### Scenario: Fields updated
- **WHEN** the user runs `ledger status <task> review --note "ready for PR"` on 2026-10-04
- **THEN** `updated` and `status_changed` are `2026-10-04` and the last history entry is `{at: 2026-10-04, to: review, note: "ready for PR"}`

### Requirement: Idempotent status
Setting a task to its current status SHALL exit 0 without writing the file or appending history.

#### Scenario: No-op
- **WHEN** a task is `active` and the user runs `ledger status <task> active`
- **THEN** the CLI exits 0 and the file's bytes and modification time are unchanged

### Requirement: Dry run
With `--dry-run`, the CLI SHALL validate the transition and report the resulting record without writing.

#### Scenario: Dry run
- **WHEN** the user runs `ledger status <task> review --dry-run --json`
- **THEN** stdout contains the record with `status: review` and the file is unchanged

### Requirement: Command alias
`ledger mv` SHALL behave identically to `ledger status`.

#### Scenario: Alias
- **WHEN** the user runs `ledger mv <task> review`
- **THEN** the result equals `ledger status <task> review`
