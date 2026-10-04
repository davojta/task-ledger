# ledger-validation Specification

## Purpose

Lets users and agents verify that every task record in the ledger is well-formed and consistent with its location, and repair the mechanical inconsistencies safely.

## Requirements

### Requirement: Check records
`ledger check` SHALL validate every task under the root and report each problem with task path, code and message. Problem codes: `parse_error`, `missing_field`, `invalid_value` (bad status, date, schema, priority or tags type), `id_mismatch`, `project_mismatch`, `orphan`.

#### Scenario: Clean ledger
- **WHEN** all records are valid
- **THEN** `ledger check` exits 0 and reports no problems

#### Scenario: Invalid status value
- **WHEN** a record has `status: wip`
- **THEN** `ledger check` reports `invalid_value` for that task and exits with code 4

### Requirement: Orphan detection
A directory at `<project>/<slug>/` (not ignored) that contains any of `input.md`, `proposal.md` or `design.md` but no `task.md` SHALL be reported as `orphan`.

#### Scenario: Orphan folder
- **WHEN** `platform/2026-08-03-ci-cache/` has `input.md` but no `task.md`
- **THEN** `ledger check` reports it as `orphan`

### Requirement: Safe fixes
`ledger check --fix` SHALL repair only `id_mismatch`, `project_mismatch` and a missing `schema` key, using the same surgical, atomic edits as other writes. It SHALL report which problems were fixed and which remain; exit code SHALL reflect remaining problems only.

#### Scenario: Fix id mismatch
- **WHEN** a record has a wrong `id` and the user runs `ledger check --fix`
- **THEN** `id` is rewritten to the path-derived ID, other lines are unchanged, and the CLI exits 0 if nothing else is wrong

#### Scenario: Unfixable problems remain
- **WHEN** a record has `status: wip` and the user runs `ledger check --fix`
- **THEN** the status is not changed and the CLI exits with code 4

### Requirement: Check output
`ledger check --json` SHALL print `{"schema_version":1,"problems":[{"path","id","code","message","fixed"}]}`.

#### Scenario: JSON problems
- **WHEN** one orphan exists and the user runs `ledger check --json`
- **THEN** `problems` has one entry with `code: "orphan"` and `fixed: false`
