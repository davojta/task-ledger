mod common;
use common::*;

use serde_json::Value;

const ID: &str = "ingest/2026-10-04-cdc-backfill";

fn new_ok(f: &Fixture, args: &[&str]) {
    f.cmd().arg("new").args(args).assert().success();
}

fn leftovers(f: &Fixture, project: &str) -> Vec<String> {
    std::fs::read_dir(f.path(project))
        .map(|rd| {
            rd.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn stdout_json(out: &[u8]) -> Value {
    serde_json::from_slice(out).expect("json stdout")
}

#[test]
fn creates_dated_task_with_builtin_files() {
    let f = Fixture::new();
    new_ok(&f, &["ingest/cdc-backfill", "--title", "CDC backfill"]);
    let task = f.read(&format!("{ID}/task.md"));
    assert!(task.contains(&format!("id: {ID}")));
    assert!(task.contains("title: \"CDC backfill\""));
    assert!(task.contains("project: ingest"));
    assert!(task.contains("status: backlog"));
    ["task", "input", "proposal", "design"]
        .iter()
        .for_each(|n| assert!(f.path(&format!("{ID}/{n}.md")).is_file(), "{n}.md"));
}

#[test]
fn human_output_names_id_and_path() {
    let f = Fixture::new();
    let out = f
        .cmd()
        .args(["new", "ingest/cdc-backfill"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with(&format!("created {ID} at ")), "{stdout}");
}

#[test]
fn no_date_prefix() {
    let f = Fixture::new();
    new_ok(&f, &["ingest/plain", "--no-date-prefix"]);
    assert!(f.path("ingest/plain/task.md").is_file());
}

#[test]
fn title_defaults_to_slug() {
    let f = Fixture::new();
    new_ok(&f, &["ingest/cdc-backfill"]);
    assert!(f
        .read(&format!("{ID}/task.md"))
        .contains("title: \"cdc-backfill\""));
}

#[test]
fn invalid_name_exits_4_and_creates_nothing() {
    let f = Fixture::new();
    f.cmd().args(["new", "Ingest/My_Task"]).assert().code(4);
    assert!(!f.path("Ingest").exists() && !f.path("ingest").exists());
}

#[test]
fn name_without_slash_exits_4() {
    let f = Fixture::new();
    f.cmd().args(["new", "ingest"]).assert().code(4);
}

#[test]
fn existing_target_exits_5_and_is_unchanged() {
    let f = Fixture::new();
    f.write(&format!("{ID}/task.md"), "keep me");
    f.cmd()
        .args(["new", "ingest/cdc-backfill"])
        .assert()
        .code(5);
    assert_eq!(f.read(&format!("{ID}/task.md")), "keep me");
    assert_eq!(leftovers(&f, "ingest"), vec!["2026-10-04-cdc-backfill"]);
}

#[test]
fn status_idea_has_single_history_entry() {
    let f = Fixture::new();
    new_ok(&f, &["ingest/x", "--status", "idea"]);
    let task = f.read("ingest/2026-10-04-x/task.md");
    assert!(task.contains("status: idea"));
    assert!(task.contains("history:\n  - {at: 2026-10-04, to: idea}\n"));
    assert_eq!(task.matches("to: ").count(), 1);
}

#[test]
fn title_with_colon_and_hash_round_trips() {
    let f = Fixture::new();
    let title = "Fix: parser # 2";
    new_ok(&f, &["ingest/x", "--title", title]);
    let out = f
        .cmd()
        .args(["show", "ingest/2026-10-04-x", "--json"])
        .output()
        .unwrap();
    assert_eq!(stdout_json(&out.stdout)["task"]["title"], title);
}

#[test]
fn user_template_is_rendered() {
    let f = Fixture::new();
    f.write("_templates/default/input.md", "# {{title}}\n");
    f.write(
        "_templates/default/task.md",
        "---\nschema: 1\nid: {{id}}\ntitle: {{title}}\nproject: {{project}}\nstatus: {{status}}\ncreated: {{date}}\nupdated: {{date}}\n---\n",
    );
    new_ok(&f, &["ingest/cdc-backfill", "--title", "CDC backfill"]);
    assert_eq!(f.read(&format!("{ID}/input.md")), "# CDC backfill\n");
    assert!(!f.path(&format!("{ID}/design.md")).exists());
}

#[test]
fn templates_dir_config_is_honored() {
    let f = Fixture::new();
    f.write("ledger.toml", "templates_dir = \"tpl\"\n");
    f.write("tpl/default/input.md", "custom {{project}}\n");
    f.write("tpl/default/task.md", &task_template());
    new_ok(&f, &["ingest/cdc-backfill"]);
    assert_eq!(f.read(&format!("{ID}/input.md")), "custom ingest\n");
}

#[test]
fn missing_named_template_exits_3() {
    let f = Fixture::new();
    f.cmd()
        .args(["new", "ingest/x", "--template", "nope"])
        .assert()
        .code(3);
    assert!(!f.path("ingest").exists());
}

#[test]
fn invalid_rendered_task_exits_4_without_leftovers() {
    let f = Fixture::new();
    f.write(
        "_templates/default/task.md",
        "---\nschema: 1\nid: {{id}}\ntitle: {{title}}\nproject: {{project}}\ncreated: {{date}}\nupdated: {{date}}\n---\n",
    );
    f.cmd().args(["new", "ingest/x"]).assert().code(4);
    assert!(!f.path("ingest/2026-10-04-x").exists());
    assert!(leftovers(&f, "ingest").is_empty());
}

#[test]
fn json_output_is_task_envelope() {
    let f = Fixture::new();
    let out = f
        .cmd()
        .args(["new", "ingest/x", "--json"])
        .output()
        .unwrap();
    let v = stdout_json(&out.stdout);
    assert_eq!(v["task"]["id"], "ingest/2026-10-04-x");
    assert_eq!(v["schema_version"], 1);
}

#[test]
fn new_task_is_listed_by_ls() {
    let f = Fixture::new();
    new_ok(&f, &["ingest/x"]);
    let out = f.cmd().args(["ls", "--json"]).output().unwrap();
    assert!(out.status.success());
    let ids: Vec<Value> = stdout_json(&out.stdout)["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].clone())
        .collect();
    assert_eq!(ids, vec![Value::from("ingest/2026-10-04-x")]);
}

fn task_template() -> String {
    "---\nschema: 1\nid: {{id}}\ntitle: {{title}}\nproject: {{project}}\nstatus: {{status}}\ncreated: {{date}}\nupdated: {{date}}\n---\n".to_string()
}
