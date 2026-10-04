# Spec Delta

## Purpose

Defines the `task.md` record that is the single source of truth for a task's metadata, and the guarantees the CLI gives whenever it edits one.

## ADDED Requirements

### Requirement: Record format
`task.md` SHALL start with a YAML frontmatter block delimited by `---` lines, followed by an optional free-text body. Required keys: `schema` (integer, `1`), `id`, `title`, `project`, `status`, `created`, `updated`. Optional keys: `stage`, `priority` (integer), `tags` (list of strings), `status_changed`, `history`. Dates SHALL be `YYYY-MM-DD`.

#### Scenario: Minimal valid record
- **WHEN** `task.md` contains frontmatter with only the required keys
- **THEN** the task loads and appears in `ledger ls`

#### Scenario: Missing frontmatter
- **WHEN** `task.md` does not start with a `---` line
- **THEN** the task is reported as invalid by `ledger check` and excluded from `ledger ls` with a warning on stderr

### Requirement: Identity fields match location
`id` SHALL equal the task's `<project>/<slug>` path and `project` SHALL equal its first segment. The location is authoritative: the CLI SHALL report the path-derived ID in all output.

#### Scenario: Mismatched id
- **WHEN** a record at `ingest/2026-09-12-cdc-backfill/` has `id: ingest/old-name`
- **THEN** `ledger ls --json` reports `id` as `ingest/2026-09-12-cdc-backfill` and `ledger check` reports the mismatch

### Requirement: History entries
`history` SHALL be a list of entries with `at` (date), `to` (status) and optional `note` (string). The CLI SHALL only append to it, never rewrite existing entries.

#### Scenario: Append preserves prior entries
- **WHEN** a task with two history entries changes status
- **THEN** the record has three entries and the first two are byte-identical to before

### Requirement: Derived stage
When `stage` is absent, the CLI SHALL report a derived stage: the last of `input`, `proposal`, `design` whose `.md` file exists in the task directory and is non-empty after trimming whitespace; `null` if none. An explicit `stage` value SHALL always win.

#### Scenario: Derived from artifacts
- **WHEN** `stage` is absent and non-empty `input.md` and `proposal.md` exist but `design.md` is empty
- **THEN** the reported stage is `proposal`

#### Scenario: Explicit stage wins
- **WHEN** `stage: implement` is set
- **THEN** the reported stage is `implement` regardless of artifacts

### Requirement: Surgical edits
When the CLI modifies `task.md`, it SHALL change only the lines of the keys it owns for that operation. All other keys, key order, comments (including trailing comments on edited lines), blank lines and the body SHALL be preserved byte-for-byte.

#### Scenario: Unknown keys and comments survive
- **WHEN** a record contains `owner: me`, a `# note` comment line and `status: active   # current`, and the user runs `ledger status <task> review`
- **THEN** the file still contains `owner: me` and `# note`, the status line reads `status: review   # current`, and the body is unchanged

#### Scenario: Result must stay parseable
- **WHEN** an edit would produce frontmatter that fails to parse
- **THEN** the CLI does not write the file and exits with code 4

### Requirement: Atomic writes
The CLI SHALL write `task.md` atomically: a reader SHALL see either the complete old file or the complete new file, never a partial one.

#### Scenario: Interrupted write
- **WHEN** the process is killed during a write
- **THEN** `task.md` holds either the previous or the new content in full
