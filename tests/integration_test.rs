use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("task-ledger").unwrap()
}

#[test]
fn test_hello_default() {
    cmd()
        .arg("hello")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello world from Rust CLI!"));
}

#[test]
fn test_hello_with_name_long() {
    cmd()
        .args(["hello", "--name", "Alice"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "hello world from Rust CLI, Alice!",
        ));
}

#[test]
fn test_hello_with_name_short() {
    cmd()
        .args(["hello", "-n", "Bob"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello world from Rust CLI, Bob!"));
}

#[test]
fn test_process_without_argument() {
    cmd()
        .arg("process")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello world from Rust CLI!"));
}

#[test]
fn test_process_with_text() {
    cmd()
        .args(["process", "Hello there"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Processed: Hello there"));
}

#[test]
fn test_process_uppercase_long() {
    cmd()
        .args(["process", "Hello there", "--uppercase"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PROCESSED: HELLO THERE"));
}

#[test]
fn test_process_uppercase_short() {
    cmd()
        .args(["process", "Hello there", "-u"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PROCESSED: HELLO THERE"));
}

#[test]
fn test_version() {
    cmd()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("task-ledger version 0.1.0"));
}

#[test]
fn test_help() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "CLI to manage spec driven artefacts",
        ));
}
