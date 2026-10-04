mod common;
use common::*;

use serde_json::{json, Value};

#[test]
fn schema_command_exits_0() {
    let fx = Fixture::new();
    let out = fx.cmd().arg("schema").output().unwrap();
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn schema_output_is_valid_json_with_definitions() {
    let fx = Fixture::new();
    let out = fx.cmd().arg("schema").output().unwrap();
    assert!(out.status.success());
    let schema: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");

    assert!(
        schema.get("$defs").is_some() || schema.get("definitions").is_some(),
        "schema must have $defs or definitions"
    );
}

#[test]
fn ls_json_output_validates_against_schema() {
    let fx = Fixture::new();
    fx.task("proj/2026-09-01-task1", "active")
        .task("proj/2026-09-02-task2", "backlog");

    let schema_out = fx.cmd().arg("schema").output().unwrap();
    let schema: Value = serde_json::from_slice(&schema_out.stdout).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    let list_out = fx.cmd().args(["ls", "--json", "--all"]).output().unwrap();
    assert!(list_out.status.success());
    let list_data: Value = serde_json::from_slice(&list_out.stdout).unwrap();

    validator.validate(&list_data).expect("ls output validates");
}

#[test]
fn show_json_output_validates_against_schema() {
    let fx = Fixture::new();
    fx.write(
        "proj/2026-09-01-task1/task.md",
        "---\nschema: 1\nid: proj/2026-09-01-task1\ntitle: Task 1\nproject: proj\nstatus: active\ncreated: 2026-09-01\nupdated: 2026-09-01\nowner: me\ntags: [task]\n---\n",
    );

    let schema_out = fx.cmd().arg("schema").output().unwrap();
    let schema: Value = serde_json::from_slice(&schema_out.stdout).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    let show_out = fx
        .cmd()
        .args(["show", "proj/2026-09-01-task1", "--json"])
        .output()
        .unwrap();
    assert!(show_out.status.success());
    let show_data: Value = serde_json::from_slice(&show_out.stdout).unwrap();

    validator
        .validate(&show_data)
        .expect("show output validates");
    assert_eq!(show_data["task"]["owner"], "me");
}

#[test]
fn check_json_output_validates_against_schema() {
    let fx = Fixture::new();
    fx.task("proj/2026-09-01-task1", "active")
        .task("proj/2026-09-02-task2", "active");

    let schema_out = fx.cmd().arg("schema").output().unwrap();
    let schema: Value = serde_json::from_slice(&schema_out.stdout).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    let check_out = fx.cmd().arg("check").arg("--json").output().unwrap();
    assert_eq!(check_out.status.code(), Some(0));
    let check_data: Value = serde_json::from_slice(&check_out.stdout).unwrap();

    validator
        .validate(&check_data)
        .expect("check output validates");
}

#[test]
fn check_with_problems_validates_despite_exit_code_4() {
    let fx = Fixture::new();
    fx.task("proj/2026-09-01-task1", "active");
    fx.write("platform/2026-08-03-orphan/input.md", "orphan content");

    let schema_out = fx.cmd().arg("schema").output().unwrap();
    let schema: Value = serde_json::from_slice(&schema_out.stdout).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    let check_out = fx.cmd().arg("check").arg("--json").output().unwrap();
    assert_eq!(check_out.status.code(), Some(4));
    let check_data: Value = serde_json::from_slice(&check_out.stdout).unwrap();

    validator
        .validate(&check_data)
        .expect("check output with problems validates");
    assert!(check_data["problems"].is_array());
}

#[test]
fn error_envelope_validates_against_schema() {
    let fx = Fixture::new();

    let schema_out = fx.cmd().arg("schema").output().unwrap();
    let schema: Value = serde_json::from_slice(&schema_out.stdout).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    let show_out = fx.cmd().args(["show", "nope", "--json"]).output().unwrap();
    assert_eq!(show_out.status.code(), Some(3));
    let error_data: Value = serde_json::from_slice(&show_out.stdout).unwrap();

    validator
        .validate(&error_data)
        .expect("error envelope validates");
    assert_eq!(error_data["error"]["code"], "not_found");
}

#[test]
fn invalid_json_does_not_validate() {
    let fx = Fixture::new();

    let schema_out = fx.cmd().arg("schema").output().unwrap();
    let schema: Value = serde_json::from_slice(&schema_out.stdout).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    let invalid = json!({"schema_version": 1, "tasks": [{"id": 1}]});
    let result = validator.validate(&invalid);

    assert!(result.is_err(), "invalid object should not validate");
}

#[test]
fn minimal_task_output_validates() {
    let fx = Fixture::new();
    fx.task("p/2026-09-01-a", "active");

    let schema_out = fx.cmd().arg("schema").output().unwrap();
    let schema: Value = serde_json::from_slice(&schema_out.stdout).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    let show_out = fx
        .cmd()
        .args(["show", "p/2026-09-01-a", "--json"])
        .output()
        .unwrap();
    assert!(show_out.status.success());
    let show_data: Value = serde_json::from_slice(&show_out.stdout).unwrap();

    validator
        .validate(&show_data)
        .expect("minimal task validates");
}
