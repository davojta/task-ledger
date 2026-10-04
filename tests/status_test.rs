mod common;

use common::*;
use predicates::prelude::*;

const ID: &str = "ingest/2026-09-12-cdc";
const FILE: &str = "ingest/2026-09-12-cdc/task.md";

fn fixture(status: &str) -> Fixture {
    let fx = Fixture::new();
    fx.task(ID, status);
    fx
}

#[test]
fn allowed_transition_updates_status() {
    let fx = fixture("active");
    fx.cmd().args(["status", ID, "review"]).assert().success();
    assert!(fx.read(FILE).contains("status: review\n"));
}

#[test]
fn illegal_transition_exits_4_and_names_allowed() {
    let fx = fixture("idea");
    let before = fx.read(FILE);
    fx.cmd()
        .args(["status", ID, "done"])
        .assert()
        .code(4)
        .stderr(predicate::str::contains("backlog, dropped"));
    assert_eq!(fx.read(FILE), before);
}

#[test]
fn reopen_requires_flag() {
    let fx = fixture("done");
    let before = fx.read(FILE);
    fx.cmd().args(["status", ID, "backlog"]).assert().code(4);
    assert_eq!(fx.read(FILE), before);
}

#[test]
fn reopen_with_flag() {
    let fx = fixture("done");
    fx.cmd()
        .args(["status", ID, "backlog", "--reopen"])
        .assert()
        .success();
    assert!(fx.read(FILE).contains("status: backlog\n"));
}

#[test]
fn force_allows_any_transition() {
    let fx = fixture("idea");
    fx.cmd()
        .args(["status", ID, "done", "--force"])
        .assert()
        .success();
    assert!(fx.read(FILE).contains("status: done\n"));
}

#[test]
fn unknown_status_is_usage_error() {
    let fx = fixture("active");
    fx.cmd().args(["status", ID, "started"]).assert().code(2);
}

#[test]
fn side_effects_with_note() {
    let fx = fixture("active");
    fx.cmd()
        .args(["status", ID, "review", "--note", "ready for PR"])
        .assert()
        .success();
    let content = fx.read(FILE);
    assert!(content.contains("updated: 2026-10-04\n"));
    assert!(content.contains("status_changed: 2026-10-04\n"));
    let history_line = content
        .lines()
        .rfind(|l| l.starts_with("  - "))
        .expect("history entry");
    assert_eq!(
        history_line,
        "  - {at: 2026-10-04, to: review, note: \"ready for PR\"}"
    );
}

#[test]
fn same_status_is_noop() {
    let fx = fixture("active");
    let path = fx.path(FILE);
    let before = std::fs::read(&path).unwrap();
    let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
    fx.cmd()
        .args(["status", ID, "active"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already active"));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), mtime);
}

#[test]
fn dry_run_json_reports_without_writing() {
    let fx = fixture("active");
    let before = fx.read(FILE);
    let out = fx
        .cmd()
        .args(["status", ID, "review", "--dry-run", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["task"]["status"], "review");
    assert_eq!(fx.read(FILE), before);
}

#[test]
fn mv_alias_matches_status() {
    let a = fixture("active");
    let b = fixture("active");
    a.cmd().args(["status", ID, "review"]).assert().success();
    b.cmd().args(["mv", ID, "review"]).assert().success();
    assert_eq!(a.read(FILE), b.read(FILE));
}

#[test]
fn ldg_binary_works() {
    let fx = fixture("active");
    fx.bin("ldg")
        .args(["status", ID, "review"])
        .assert()
        .success();
    assert!(fx.read(FILE).contains("status: review\n"));
}

#[test]
fn edit_is_surgical() {
    let fx = Fixture::new();
    let original = "---\nschema: 1\nid: ingest/2026-09-12-cdc\ntitle: CDC\nproject: ingest\n# note\nstatus: active   # current\nowner: me\ncreated: 2026-09-12\nupdated: 2026-09-12\nhistory:\n  - {at: 2026-09-12, to: active}\n---\nBody\nstatus: active\n";
    fx.write(FILE, original);
    fx.cmd().args(["status", ID, "review"]).assert().success();
    let expected = "---\nschema: 1\nid: ingest/2026-09-12-cdc\ntitle: CDC\nproject: ingest\n# note\nstatus: review   # current\nowner: me\ncreated: 2026-09-12\nupdated: 2026-10-04\nhistory:\n  - {at: 2026-09-12, to: active}\n  - {at: 2026-10-04, to: review}\nstatus_changed: 2026-10-04\n---\nBody\nstatus: active\n";
    assert_eq!(fx.read(FILE), expected);
}

#[test]
fn not_found_json_error() {
    let fx = fixture("active");
    let out = fx
        .cmd()
        .args(["status", "nope", "review", "--json"])
        .assert()
        .code(3)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["error"]["code"], "not_found");
}
