# Project: task-ledger

CLI to manage spec driven artefacts for the task. Built with Cargo, clap, clippy, and rustfmt.

## Tech Stack

- **Runtime**: Rust (stable, edition 2021)
- **Package Manager**: Cargo
- **CLI Framework**: clap (derive feature)
- **Testing**: built-in `cargo test` + `assert_cmd` for integration tests
- **Lint**: clippy
- **Format**: rustfmt

## Commands

```bash
make build                  # Build the project
make run-main               # Run CLI hello command
make run-test               # Unit tests (lib only)
make run-integration-tests  # Integration tests
make run-all-tests          # All tests
make run-lint               # Lint with clippy
make run-format             # Format with rustfmt
make check                  # Lint + format check (CI target)
```

Direct cargo commands:

```bash
cargo run -- hello
cargo run -- hello --name "Alice"
cargo run -- process "Hello there"
cargo run -- process "Hello there" --uppercase
cargo run -- version
```

## Project Structure

```
task-ledger/
├── src/
│   ├── main.rs        # clap CLI entry point
│   └── lib.rs         # Business logic + inline unit tests
├── tests/
│   └── integration_test.rs  # CLI integration tests via assert_cmd
├── Cargo.toml
├── Cargo.lock
└── Makefile
```

## Validation

```bash
# After each non-trivial edit
make run-all-tests && make run-lint

# Before committing
make check
```
