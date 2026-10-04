mod common;
use common::*;

use serde_json::Value;

fn parse(out: &[u8]) -> Value {
    serde_json::from_slice(out).expect("valid json")
}

fn ids(v: &Value) -> Vec<String> {
    v["tasks"]
        .as_array()
        .expect("tasks array")
        .iter()
        .map(|t| t["id"].as_str().expect("id").to_string())
        .collect()
}

fn ls_ids(fx: &Fixture, args: &[&str]) -> Vec<String> {
    let out = fx
        .cmd()
        .arg("ls")
        .args(args)
        .arg("--json")
        .output()
        .unwrap();
    assert!(out.status.success());
    ids(&parse(&out.stdout))
}

fn task_with(id: &str, status: &str, extra: &str, created: &str, updated: &str) -> String {
    let project = id.split('/').next().unwrap();
    format!(
        "---\nschema: 1\nid: {id}\ntitle: T {id}\nproject: {project}\nstatus: {status}\ncreated: {created}\nupdated: {updated}\n{extra}---\nbody of {id}\n"
    )
}

fn two_tasks() -> Fixture {
    let fx = Fixture::new();
    fx.task("ingest/a", "active").task("ingest/b", "done");
    fx
}

#[test]
fn default_hides_done() {
    assert_eq!(ls_ids(&two_tasks(), &[]), ["ingest/a"]);
}

#[test]
fn all_includes_done() {
    assert_eq!(ls_ids(&two_tasks(), &["--all"]), ["ingest/a", "ingest/b"]);
}

#[test]
fn status_done_shows_only_done() {
    assert_eq!(ls_ids(&two_tasks(), &["--status", "done"]), ["ingest/b"]);
}

#[test]
fn combined_project_and_tag() {
    let fx = Fixture::new();
    fx.write(
        "ingest/a/task.md",
        &task_with(
            "ingest/a",
            "active",
            "tags: [task]\n",
            "2026-09-01",
            "2026-09-01",
        ),
    )
    .task("ingest/b", "active")
    .write(
        "other/c/task.md",
        &task_with(
            "other/c",
            "active",
            "tags: [task]\n",
            "2026-09-01",
            "2026-09-01",
        ),
    );
    assert_eq!(
        ls_ids(&fx, &["--project", "ingest", "--tag", "task"]),
        ["ingest/a"]
    );
}

#[test]
fn stage_filter_uses_derived_stage() {
    let fx = Fixture::new();
    fx.task("p/a", "active").task("p/b", "active");
    fx.write("p/a/proposal.md", "x");
    assert_eq!(ls_ids(&fx, &["--stage", "proposal"]), ["p/a"]);
}

#[test]
fn sort_updated_newest_first() {
    let fx = Fixture::new();
    fx.write(
        "p/a/task.md",
        &task_with("p/a", "active", "", "2026-09-01", "2026-09-02"),
    )
    .write(
        "p/b/task.md",
        &task_with("p/b", "active", "", "2026-09-01", "2026-09-10"),
    );
    assert_eq!(ls_ids(&fx, &["--sort", "updated"]), ["p/b", "p/a"]);
    assert_eq!(ls_ids(&fx, &[]), ["p/a", "p/b"]);
}

#[test]
fn sort_created_newest_first() {
    let fx = Fixture::new();
    fx.write(
        "p/a/task.md",
        &task_with("p/a", "active", "", "2026-09-01", "2026-09-02"),
    )
    .write(
        "p/b/task.md",
        &task_with("p/b", "active", "", "2026-09-05", "2026-09-02"),
    );
    assert_eq!(ls_ids(&fx, &["--sort", "created"]), ["p/b", "p/a"]);
}

#[test]
fn sort_status_follows_lifecycle_order() {
    let fx = Fixture::new();
    fx.task("p/a", "review")
        .task("p/b", "idea")
        .task("p/c", "active")
        .task("p/d", "backlog")
        .task("p/e", "active");
    assert_eq!(
        ls_ids(&fx, &["--sort", "status"]),
        ["p/b", "p/d", "p/c", "p/e", "p/a"]
    );
}

#[test]
fn paths_are_absolute_and_exist() {
    let fx = two_tasks();
    let out = fx.cmd().args(["ls", "--all", "--paths"]).output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    for line in lines {
        let p = std::path::Path::new(line);
        assert!(p.is_absolute() && p.is_dir(), "{line}");
    }
}

#[test]
fn jsonl_lines_are_task_objects() {
    let fx = two_tasks();
    let out = fx.cmd().args(["ls", "--all", "--jsonl"]).output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let got: Vec<String> = text
        .lines()
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["id"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(got, ["ingest/a", "ingest/b"]);
}

#[test]
fn json_envelope_has_schema_version() {
    let fx = two_tasks();
    let out = fx.cmd().args(["ls", "--json"]).output().unwrap();
    assert_eq!(parse(&out.stdout)["schema_version"], 1);
}

#[test]
fn empty_ledger_outputs() {
    let fx = Fixture::new();
    let out = fx.cmd().args(["ls", "--json"]).output().unwrap();
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        r#"{"schema_version":1,"tasks":[]}"#
    );
    let out = fx.cmd().arg("ls").output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(text.lines().count(), 1);
    assert!(text.starts_with("ID"));
}

#[test]
fn human_table_has_header_and_rows() {
    let fx = two_tasks();
    let out = fx.cmd().arg("ls").output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let mut lines = text.lines();
    let header: Vec<_> = lines.next().unwrap().split_whitespace().collect();
    assert_eq!(header, ["ID", "STATUS", "STAGE", "UPDATED", "TITLE"]);
    let row: Vec<_> = lines.next().unwrap().split_whitespace().collect();
    assert_eq!(&row[..4], ["ingest/a", "active", "-", "2026-09-01"]);
}

#[test]
fn broken_record_is_skipped_with_warning() {
    let fx = Fixture::new();
    fx.task("p/a", "active")
        .task("p/b", "active")
        .write("p/c/task.md", "---\nnot: [valid\n---\n");
    let out = fx.cmd().args(["ls", "--json"]).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(ids(&parse(&out.stdout)), ["p/a", "p/b"]);
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("warning") && err.contains("p/c"), "{err}");
}

#[test]
fn ldg_output_equals_ledger() {
    let fx = two_tasks();
    let a = fx.cmd().args(["ls", "--json"]).output().unwrap();
    let b = fx.bin("ldg").args(["ls", "--json"]).output().unwrap();
    assert!(a.status.success());
    assert_eq!(a.stdout, b.stdout);
}

#[test]
fn unknown_frontmatter_key_in_json() {
    let fx = Fixture::new();
    fx.write(
        "p/a/task.md",
        &task_with("p/a", "active", "owner: me\n", "2026-09-01", "2026-09-01"),
    );
    let out = fx.cmd().args(["ls", "--json"]).output().unwrap();
    assert_eq!(parse(&out.stdout)["tasks"][0]["owner"], "me");
}

fn show_fixture() -> Fixture {
    let fx = Fixture::new();
    fx.task("proj/2026-10-01-alpha", "active")
        .task("proj/2026-10-01-beta-one", "active")
        .task("proj/2026-10-01-gamma-one", "active");
    fx.write("proj/2026-10-01-alpha/input.md", "in")
        .write("proj/2026-10-01-alpha/proposal.md", "prop");
    fx
}

#[test]
fn show_reports_artifacts() {
    let fx = show_fixture();
    let out = fx
        .cmd()
        .args(["show", "proj/2026-10-01-alpha", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v = parse(&out.stdout);
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["task"]["id"], "proj/2026-10-01-alpha");
    assert_eq!(
        v["task"]["artifacts"],
        serde_json::json!({"input": true, "proposal": true, "design": false})
    );
}

#[test]
fn show_by_unique_slug_suffix() {
    let fx = show_fixture();
    let out = fx.cmd().args(["show", "alpha", "--json"]).output().unwrap();
    assert!(out.status.success());
    assert_eq!(parse(&out.stdout)["task"]["id"], "proj/2026-10-01-alpha");
}

#[test]
fn show_dot_in_task_dir() {
    let fx = show_fixture();
    let out = fx
        .cmd()
        .current_dir(fx.path("proj/2026-10-01-alpha"))
        .args(["show", ".", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(parse(&out.stdout)["task"]["id"], "proj/2026-10-01-alpha");
}

#[test]
fn show_not_found_json() {
    let fx = show_fixture();
    let out = fx.cmd().args(["show", "nope", "--json"]).output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(parse(&out.stdout)["error"]["code"], "not_found");
    assert!(!out.stderr.is_empty());
}

#[test]
fn show_ambiguous_suffix() {
    let fx = show_fixture();
    let out = fx.cmd().args(["show", "one", "--json"]).output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(parse(&out.stdout)["error"]["code"], "ambiguous");
}

#[test]
fn show_broken_record_exits_4() {
    let fx = Fixture::new();
    fx.write("p/bad/task.md", "---\nnot: [valid\n---\n");
    let out = fx.cmd().args(["show", "p/bad"]).output().unwrap();
    assert_eq!(out.status.code(), Some(4));
}

#[test]
fn show_human_and_body() {
    let fx = Fixture::new();
    fx.write(
        "p/a/task.md",
        &task_with("p/a", "active", "", "2026-09-01", "2026-09-01"),
    );
    let plain = fx.cmd().args(["show", "p/a"]).output().unwrap();
    let text = String::from_utf8(plain.stdout).unwrap();
    assert!(text.contains("id: p/a\n"));
    assert!(text.contains("artifacts: input no proposal no design no"));
    assert!(!text.contains("body of p/a"));
    let with = fx.cmd().args(["show", "p/a", "--body"]).output().unwrap();
    let text = String::from_utf8(with.stdout).unwrap();
    assert!(text.ends_with("\n\nbody of p/a\n"), "{text:?}");
}

#[test]
fn missing_root_exits_3() {
    let empty = tempfile::TempDir::new().unwrap();
    let out = assert_cmd::Command::cargo_bin("ledger")
        .unwrap()
        .env("LEDGER_ROOT", empty.path())
        .current_dir(empty.path())
        .args(["ls"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
}
