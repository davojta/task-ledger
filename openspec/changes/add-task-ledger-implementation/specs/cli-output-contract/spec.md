# Spec Delta

## Purpose

Defines the stable, non-interactive interface that humans and agents rely on: binary names, JSON shapes, error reporting and exit codes.

## ADDED Requirements

### Requirement: Binaries
The CLI SHALL be installed as two binaries, `ledger` and `ldg`, with identical commands, flags and output. `--version` SHALL print the package version.

#### Scenario: Alias binary
- **WHEN** the user runs `ldg ls --json`
- **THEN** the output equals `ledger ls --json`

### Requirement: Never prompt
No command SHALL read from stdin or prompt for input; all input SHALL come from arguments, flags and environment.

#### Scenario: Non-TTY execution
- **WHEN** an agent runs any command with stdin closed
- **THEN** the command completes without blocking

### Requirement: JSON envelopes
With `--json`, `ls` SHALL print `{"schema_version":1,"tasks":[...]}` and `show`, `new` and `status` SHALL print `{"schema_version":1,"task":{...}}`. A task object SHALL contain every frontmatter key (unknown keys included) plus `path` (absolute task directory) and `artifacts` (`input`, `proposal`, `design` booleans); `id`, `project` and `stage` SHALL be the reported (path-derived / derived) values. Fields MAY be added; renaming or removing a field SHALL require a new `schema_version`.

#### Scenario: Show envelope
- **WHEN** the user runs `ledger show <task> --json`
- **THEN** stdout parses as JSON with `schema_version` 1 and a `task` object containing `id`, `status`, `path` and `artifacts`

#### Scenario: Unknown keys exposed
- **WHEN** a record contains `owner: me`
- **THEN** the task object contains `"owner": "me"`

### Requirement: Errors
Errors SHALL be printed as human text on stderr. With `--json`, the CLI SHALL also print `{"error":{"code":"<code>","message":"<text>"}}` on stdout, where code is one of `internal`, `not_found`, `ambiguous`, `invalid`, `illegal_transition`, `conflict`.

#### Scenario: JSON not found
- **WHEN** the user runs `ledger show nope --json`
- **THEN** stdout is `{"error":{"code":"not_found",...}}`, stderr has a message, and the exit code is 3

### Requirement: Exit codes
The CLI SHALL exit `0` on success, `1` on internal errors (I/O and unexpected failures), `2` on usage errors, `3` when a root, task or template is not found or a reference is ambiguous, `4` on validation failures or illegal transitions, `5` when a target already exists.

#### Scenario: Usage error
- **WHEN** the user runs `ledger status` with no arguments
- **THEN** the CLI exits with code 2

### Requirement: Output schema command
`ledger schema` SHALL print a JSON Schema document describing the list envelope, the single-task envelope, the check envelope and the error object, generated from the same types used to produce output.

#### Scenario: Schema matches output
- **WHEN** `ledger ls --json` output is validated against `ledger schema`
- **THEN** validation succeeds
