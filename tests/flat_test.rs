mod common;
use common::*;

use predicates::str::contains;
use serde_json::Value;

const ID: &str = "2026-10-04-grand-plan";

/// A ledger whose `ledger.toml` sets `project`, so tasks live at `<root>/<slug>/`.
fn flat() -> Fixture {
    let f = Fixture::new();
    f.write("ledger.toml", "project = \"anki-helpers\"\n");
    f
}

fn stdout_json(out: &[u8]) -> Value {
    serde_json::from_slice(out).expect("json stdout")
}

#[test]
fn new_creates_task_at_root_with_configured_project() {
    let f = flat();
    f.cmd()
        .args(["new", "grand-plan", "--title", "Grand plan"])
        .assert()
        .success();
    let task = f.read(&format!("{ID}/task.md"));
    assert!(task.contains(&format!("id: {ID}\n")));
    assert!(task.contains("project: anki-helpers\n"));
    assert!(!f.path("anki-helpers").exists());
}

#[test]
fn new_rejects_project_prefix() {
    let f = flat();
    f.cmd()
        .args(["new", "other/grand-plan"])
        .assert()
        .code(4)
        .stderr(contains("expected <slug>"))
        .stderr(contains("anki-helpers"));
}

#[test]
fn ls_show_status_and_check_work_on_flat_tasks() {
    let f = flat();
    f.cmd().args(["new", "grand-plan"]).assert().success();

    let out = f.cmd().args(["ls", "--json"]).output().unwrap();
    let tasks = &stdout_json(&out.stdout)["tasks"];
    assert_eq!(tasks[0]["id"], ID);
    assert_eq!(tasks[0]["project"], "anki-helpers");

    f.cmd().args(["show", ID]).assert().success();
    f.cmd()
        .args(["status", "grand-plan", "active"])
        .assert()
        .success();
    assert!(f.read(&format!("{ID}/task.md")).contains("status: active"));
    f.cmd().arg("check").assert().success();
}

#[test]
fn check_fixes_project_mismatch_to_configured_project() {
    let f = flat();
    f.write(
        &format!("{ID}/task.md"),
        &format!("---\nschema: 1\nid: {ID}\ntitle: T\nproject: plan\nstatus: backlog\ncreated: 2026-09-01\nupdated: 2026-09-01\n---\n"),
    );
    f.cmd().arg("check").assert().code(4);
    f.cmd().args(["check", "--fix"]).assert().success();
    assert!(f
        .read(&format!("{ID}/task.md"))
        .contains("project: anki-helpers\n"));
}

#[test]
fn invalid_project_in_config_is_rejected() {
    let f = Fixture::new();
    f.write("ledger.toml", "project = \"Anki Helpers\"\n");
    f.cmd()
        .arg("ls")
        .assert()
        .code(4)
        .stderr(contains("ledger.toml"));
}
