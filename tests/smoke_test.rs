mod common;

use common::Fixture;
use predicates::prelude::*;

#[test]
fn both_binaries_print_version() {
    let fx = Fixture::new();
    for bin in ["ledger", "ldg"] {
        fx.bin(bin)
            .arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
    }
}

#[test]
fn fixture_writes_task() {
    let fx = Fixture::new();
    fx.task("ingest/2026-09-12-cdc", "active");
    assert!(fx
        .read("ingest/2026-09-12-cdc/task.md")
        .contains("status: active"));
}

#[test]
fn missing_subcommand_is_usage_error() {
    Fixture::new().cmd().assert().code(2);
}
