# Agent Integration

Find tasks with `ledger ls --json` (add `--all` for done/dropped); one task: `ledger show <task> --json`.

Change status only via `ledger status <task> <status>` (alias `mv`). Statuses: idea→backlog→active→review→done; review→active; any non-terminal→dropped; done/dropped→backlog with `--reopen`; `--force` overrides. Use `--note "text"` to record intent.

Create tasks with `ledger new <project>/<slug> --title "..."`.

Never move or rename task folders; never hand-edit `status` or `history` in task.md.

Exit codes: 0 success, 1 internal, 2 usage, 3 not found/ambiguous, 4 invalid/illegal transition, 5 already exists. With `--json`, errors are `{"error":{"code":"...","message":"..."}}` on stdout.

`ledger schema` prints the JSON Schema of all `--json` output.
