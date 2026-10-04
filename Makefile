.PHONY: help build run-main run-test run-integration-tests run-all-tests run-lint run-format check clean

help:
	@echo "task-ledger - Available Commands:"
	@echo "  make build                  - Build the project"
	@echo "  make run-main               - Run the CLI hello command"
	@echo "  make run-test               - Run unit tests"
	@echo "  make run-integration-tests  - Run integration tests"
	@echo "  make run-all-tests          - Run all tests"
	@echo "  make run-lint               - Lint with clippy"
	@echo "  make run-format             - Format with rustfmt"
	@echo "  make check                  - Lint + format check (CI target)"
	@echo "  make clean                  - Clean build artifacts"

build:
	cargo build

run-main:
	cargo run -- hello

run-test:
	cargo test --lib

run-integration-tests:
	cargo test --tests

run-all-tests:
	cargo test

run-lint:
	cargo clippy -- -D warnings

run-format:
	cargo fmt

check:
	@echo "Running clippy..."
	@cargo clippy -- -D warnings
	@echo "Checking formatting..."
	@cargo fmt --check
	@echo "All checks passed!"

clean:
	cargo clean
