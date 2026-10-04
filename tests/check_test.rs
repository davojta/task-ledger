mod common;
use common::*;

use serde_json::Value;

const ID: &str = "ingest/2026-09-12-cdc";

fn json_problems(f: &Fixture, args: &[&str]) -> (Vec<Value>, i32) {
    let out = f
        .cmd()
        .arg("--json")
        .arg("check")
        .args(args)
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["schema_version"], 1);
    (
        v["problems"].as_array().unwrap().clone(),
        out.status.code().unwrap(),
    )
}

#[test]
fn clean_ledger_exits_0_with_ok_message() {
    let f = Fixture::new();
    f.task(ID, "active");
    f.cmd()
        .arg("check")
        .assert()
        .code(0)
        .stdout("ok: 1 task checked, no problems\n");
}

#[test]
fn invalid_status_exits_4() {
    let f = Fixture::new();
    f.task(ID, "wip");
    let (problems, code) = json_problems(&f, &[]);
    assert_eq!(code, 4);
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0]["code"], "invalid_value");
    f.cmd()
        .arg("check")
        .assert()
        .code(4)
        .stdout(predicates::str::starts_with(format!(
            "{ID}: invalid_value invalid status `wip`"
        )));
}

#[test]
fn orphan_reported_in_json() {
    let f = Fixture::new();
    f.write("platform/2026-08-03-ci-cache/input.md", "x\n");
    let (problems, code) = json_problems(&f, &[]);
    assert_eq!(code, 4);
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0]["code"], "orphan");
    assert_eq!(problems[0]["fixed"], false);
    assert_eq!(problems[0]["id"], "platform/2026-08-03-ci-cache");
    assert_eq!(
        problems[0]["path"],
        f.path("platform/2026-08-03-ci-cache").display().to_string()
    );
}

#[test]
fn fix_rewrites_wrong_id_only() {
    let f = Fixture::new();
    let rel = format!("{ID}/task.md");
    f.write(&rel, &wrong_id("active"));
    f.cmd().args(["check", "--fix"]).assert().code(0);
    assert_eq!(f.read(&rel), task_md(ID, "active"));
}

#[test]
fn fix_rewrites_wrong_project() {
    let f = Fixture::new();
    let rel = format!("{ID}/task.md");
    f.write(
        &rel,
        &task_md(ID, "active").replace("project: ingest", "project: other"),
    );
    f.cmd().args(["check", "--fix"]).assert().code(0);
    assert_eq!(f.read(&rel), task_md(ID, "active"));
}

#[test]
fn fix_inserts_missing_schema_before_closing() {
    let f = Fixture::new();
    let rel = format!("{ID}/task.md");
    f.write(&rel, &task_md(ID, "active").replace("schema: 1\n", ""));
    f.cmd().args(["check", "--fix"]).assert().code(0);
    assert_eq!(
        f.read(&rel),
        task_md(ID, "active").replace("schema: 1\n", "").replacen(
            "updated: 2026-09-01\n---",
            "updated: 2026-09-01\nschema: 1\n---",
            1
        )
    );
}

#[test]
fn fix_leaves_invalid_status_and_exits_4() {
    let f = Fixture::new();
    let rel = format!("{ID}/task.md");
    f.task(ID, "wip");
    f.cmd().args(["check", "--fix"]).assert().code(4);
    assert_eq!(f.read(&rel), task_md(ID, "wip"));
}

#[test]
fn mixed_fixes_id_and_keeps_invalid_value() {
    let f = Fixture::new();
    let rel = format!("{ID}/task.md");
    f.write(&rel, &wrong_id("wip"));
    let (problems, code) = json_problems(&f, &["--fix"]);
    assert_eq!(code, 4);
    let flags: Vec<_> = problems
        .iter()
        .map(|p| (p["code"].as_str().unwrap(), p["fixed"].as_bool().unwrap()))
        .collect();
    assert_eq!(flags, [("invalid_value", false), ("id_mismatch", true)]);
    assert_eq!(f.read(&rel), task_md(ID, "wip"));
}

#[test]
fn missing_frontmatter_is_parse_error() {
    let f = Fixture::new();
    f.write(&format!("{ID}/task.md"), "just text\n");
    let (problems, code) = json_problems(&f, &[]);
    assert_eq!(code, 4);
    assert_eq!(problems[0]["code"], "parse_error");
}

#[test]
fn json_problem_shape() {
    let f = Fixture::new();
    f.write(&format!("{ID}/task.md"), &wrong_id("active"));
    let out = f.cmd().args(["--json", "check"]).output().unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let expected = serde_json::json!({
        "schema_version": 1,
        "problems": [{
            "path": f.path(ID).display().to_string(),
            "id": ID,
            "code": "id_mismatch",
            "message": format!("id `ingest/wrong` does not match location `{ID}`"),
            "fixed": false
        }]
    });
    assert_eq!(v, expected);
}

#[test]
fn fixed_problem_is_marked_in_human_output() {
    let f = Fixture::new();
    f.write(&format!("{ID}/task.md"), &wrong_id("active"));
    f.cmd()
        .args(["check", "--fix"])
        .assert()
        .code(0)
        .stdout(format!(
            "{ID}: id_mismatch id `ingest/wrong` does not match location `{ID}` (fixed)\n"
        ));
}

#[test]
fn templates_are_ignored() {
    let f = Fixture::new();
    f.write("_templates/default/task.md", "not a record\n");
    f.cmd().arg("check").assert().code(0);
}

fn wrong_id(status: &str) -> String {
    task_md(ID, status).replace(&format!("id: {ID}"), "id: ingest/wrong")
}
