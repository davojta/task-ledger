use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::core::date::is_valid_date;
use crate::core::error::{LedgerError, Result};
use crate::core::lifecycle::Status;
use crate::core::scan::{TaskDir, ARTIFACT_NAMES};

pub const KNOWN_KEYS: [&str; 12] = [
    "schema",
    "id",
    "title",
    "project",
    "status",
    "stage",
    "priority",
    "tags",
    "created",
    "updated",
    "status_changed",
    "history",
];

/// Declared in report order; `Ord` follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProblemCode {
    ParseError,
    MissingField,
    InvalidValue,
    IdMismatch,
    ProjectMismatch,
    Orphan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordProblem {
    pub code: ProblemCode,
    pub message: String,
}

pub fn missing_field_message(key: &str) -> String {
    format!("missing required field `{key}`")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HistoryEntry {
    pub at: String,
    pub to: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Validated frontmatter. `raw` keeps every key (known and unknown) in file order.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub schema: i64,
    pub id: String,
    pub title: String,
    pub project: String,
    pub status: Status,
    pub stage: Option<String>,
    pub priority: Option<i64>,
    pub tags: Vec<String>,
    pub created: String,
    pub updated: String,
    pub status_changed: Option<String>,
    pub history: Vec<HistoryEntry>,
    pub raw: Map<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, JsonSchema)]
pub struct Artifacts {
    pub input: bool,
    pub proposal: bool,
    pub design: bool,
}

/// A loaded task: location + record + derived values.
#[derive(Debug, Clone)]
pub struct Task {
    pub location: TaskDir,
    pub record: Record,
    pub artifacts: Artifacts,
    /// Explicit `stage` if set, else derived from artifacts.
    pub stage: Option<String>,
    pub body: String,
}

/// Split `task.md` into (frontmatter yaml, body). The file must start with a
/// `---` line; the frontmatter ends at the next line equal to `---`.
pub fn split_frontmatter(content: &str) -> std::result::Result<(String, String), RecordProblem> {
    let parse_error = |message: &str| RecordProblem {
        code: ProblemCode::ParseError,
        message: message.to_string(),
    };
    let mut lines = content.split_inclusive('\n');
    if lines.next().map(str::trim_end) != Some("---") {
        return Err(parse_error(
            "task.md must start with a `---` frontmatter line",
        ));
    }
    let mut yaml = String::new();
    for line in lines.by_ref() {
        if line.trim_end() == "---" {
            return Ok((yaml, lines.collect()));
        }
        yaml.push_str(line);
    }
    Err(parse_error("frontmatter has no closing `---` line"))
}

/// Parse frontmatter YAML into a JSON map (key order preserved).
/// Empty frontmatter is an empty map.
pub fn parse_yaml(yaml: &str) -> std::result::Result<Map<String, Value>, RecordProblem> {
    let parse_error = |message: String| RecordProblem {
        code: ProblemCode::ParseError,
        message,
    };
    match serde_saphyr::from_str::<Value>(yaml) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(Value::Null) => Ok(Map::new()),
        Ok(_) => Err(parse_error("frontmatter must be a YAML mapping".into())),
        Err(e) => Err(parse_error(format!("invalid frontmatter YAML: {e}"))),
    }
}

type Check<T> = std::result::Result<T, RecordProblem>;

fn problem(code: ProblemCode, message: String) -> RecordProblem {
    RecordProblem { code, message }
}

fn invalid(message: String) -> RecordProblem {
    problem(ProblemCode::InvalidValue, message)
}

fn present<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    map.get(key).filter(|value| !value.is_null())
}

fn required<'a>(map: &'a Map<String, Value>, key: &str) -> Check<&'a Value> {
    present(map, key).ok_or_else(|| problem(ProblemCode::MissingField, missing_field_message(key)))
}

fn optional<T>(
    map: &Map<String, Value>,
    key: &str,
    parse: impl Fn(&str, &Value) -> Check<T>,
) -> Check<Option<T>> {
    present(map, key).map(|value| parse(key, value)).transpose()
}

fn show(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_string)
}

fn string_value(key: &str, value: &Value) -> Check<String> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| invalid(format!("`{key}` must be a string, got `{}`", show(value))))
}

fn date_value(key: &str, value: &Value) -> Check<String> {
    string_value(key, value).and_then(|date| {
        if is_valid_date(&date) {
            Ok(date)
        } else {
            Err(invalid(format!("invalid date in `{key}`: `{date}`")))
        }
    })
}

fn status_value(_key: &str, value: &Value) -> Check<Status> {
    value.as_str().and_then(Status::parse).ok_or_else(|| {
        let expected = Status::ALL.map(Status::as_str).join(", ");
        invalid(format!(
            "invalid status `{}`; expected one of {expected}",
            show(value)
        ))
    })
}

fn schema_value(key: &str, value: &Value) -> Check<i64> {
    match value.as_i64() {
        Some(1) => Ok(1),
        _ => Err(invalid(format!("`{key}` must be 1"))),
    }
}

fn priority_value(key: &str, value: &Value) -> Check<i64> {
    value
        .as_i64()
        .ok_or_else(|| invalid(format!("`{key}` must be an integer, got `{}`", show(value))))
}

fn tags_value(key: &str, value: &Value) -> Check<Vec<String>> {
    value
        .as_array()
        .ok_or_else(|| invalid(format!("`{key}` must be a list of strings")))?
        .iter()
        .map(|tag| string_value("tags", tag))
        .collect()
}

fn history_entry(value: &Value) -> Check<HistoryEntry> {
    let entry = serde_json::from_value::<HistoryEntry>(value.clone())
        .map_err(|e| invalid(format!("invalid `history` entry: {e}")))?;
    date_value("history.at", &Value::String(entry.at.clone()))?;
    Ok(entry)
}

fn history_value(key: &str, value: &Value) -> Check<Vec<HistoryEntry>> {
    value
        .as_array()
        .ok_or_else(|| invalid(format!("`{key}` must be a list of mappings")))?
        .iter()
        .map(history_entry)
        .collect()
}

fn req<T>(
    map: &Map<String, Value>,
    key: &str,
    parse: impl Fn(&str, &Value) -> Check<T>,
    problems: &mut Vec<RecordProblem>,
) -> Option<T> {
    keep(required(map, key).and_then(|v| parse(key, v)), problems)
}

fn keep<T>(result: Check<T>, problems: &mut Vec<RecordProblem>) -> Option<T> {
    result.map_err(|problem| problems.push(problem)).ok()
}

/// Validate known keys of a parsed map. Collects every problem
/// (`MissingField` / `InvalidValue`), not just the first.
pub fn validate(map: Map<String, Value>) -> std::result::Result<Record, Vec<RecordProblem>> {
    let mut problems = Vec::new();
    let schema = req(&map, "schema", schema_value, &mut problems);
    let id = req(&map, "id", string_value, &mut problems);
    let title = req(&map, "title", string_value, &mut problems);
    let project = req(&map, "project", string_value, &mut problems);
    let status = req(&map, "status", status_value, &mut problems);
    let created = req(&map, "created", date_value, &mut problems);
    let updated = req(&map, "updated", date_value, &mut problems);
    let stage = keep(optional(&map, "stage", string_value), &mut problems);
    let priority = keep(optional(&map, "priority", priority_value), &mut problems);
    let tags = keep(optional(&map, "tags", tags_value), &mut problems);
    let status_changed = keep(optional(&map, "status_changed", date_value), &mut problems);
    let history = keep(optional(&map, "history", history_value), &mut problems);
    let build = move || -> Option<Record> {
        Some(Record {
            schema: schema?,
            id: id?,
            title: title?,
            project: project?,
            status: status?,
            stage: stage?,
            priority: priority?,
            tags: tags?.unwrap_or_default(),
            created: created?,
            updated: updated?,
            status_changed: status_changed?,
            history: history?.unwrap_or_default(),
            raw: map,
        })
    };
    match build() {
        Some(record) if problems.is_empty() => Ok(record),
        _ => Err(problems),
    }
}

/// split + parse + validate.
pub fn parse_record(content: &str) -> std::result::Result<(Record, String), Vec<RecordProblem>> {
    let (yaml, body) = split_frontmatter(content).map_err(|p| vec![p])?;
    let map = parse_yaml(&yaml).map_err(|p| vec![p])?;
    validate(map).map(|record| (record, body))
}

/// Present = `<name>.md` exists and is non-empty after trimming whitespace.
pub fn artifacts(dir: &Path) -> Artifacts {
    let [input, proposal, design] = ARTIFACT_NAMES.map(|name| {
        std::fs::read_to_string(dir.join(format!("{name}.md")))
            .is_ok_and(|content| !content.trim().is_empty())
    });
    Artifacts {
        input,
        proposal,
        design,
    }
}

/// Explicit stage wins; else last present of input/proposal/design; else None.
pub fn derive_stage(explicit: Option<&str>, artifacts: Artifacts) -> Option<String> {
    explicit.map(str::to_string).or_else(|| {
        let present = [artifacts.input, artifacts.proposal, artifacts.design];
        ARTIFACT_NAMES
            .into_iter()
            .zip(present)
            .rfind(|(_, present)| *present)
            .map(|(name, _)| name.to_string())
    })
}

/// Read and parse `task.md` of `location`. Parse/validation problems become
/// `LedgerError::Invalid` naming the path and listing the problems.
pub fn load_task(location: &TaskDir) -> Result<Task> {
    let path = location.task_file();
    let content = std::fs::read_to_string(&path).map_err(|e| LedgerError::io(&path, e))?;
    let (record, body) = parse_record(&content).map_err(|problems| {
        let messages = problems
            .iter()
            .map(|p| p.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        LedgerError::Invalid(format!("{}: {messages}", path.display()))
    })?;
    let artifacts = artifacts(&location.dir);
    let stage = derive_stage(record.stage.as_deref(), artifacts);
    Ok(Task {
        location: location.clone(),
        record,
        artifacts,
        stage,
        body,
    })
}

#[cfg(test)]
mod split_parse_tests {
    use super::*;

    const SAMPLE: &str = "---\nschema: 1\ncreated: 2026-09-12\nstatus: active   # current\nowner: me\nhistory:\n  - {at: 2026-09-12, to: backlog}\n---\nBody line\n";

    #[test]
    fn splits_and_parses_in_order() {
        let (yaml, body) = split_frontmatter(SAMPLE).unwrap();
        assert_eq!(body, "Body line\n");
        let map = parse_yaml(&yaml).unwrap();
        let keys: Vec<&str> = map.keys().map(String::as_str).collect();
        assert_eq!(keys, ["schema", "created", "status", "owner", "history"]);
        assert_eq!(map["schema"], 1);
        assert_eq!(map["created"], "2026-09-12");
        assert_eq!(map["status"], "active");
        assert_eq!(map["history"][0]["to"], "backlog");
    }

    #[test]
    fn missing_or_unclosed_frontmatter() {
        assert_eq!(
            split_frontmatter("no\n").unwrap_err().code,
            ProblemCode::ParseError
        );
        assert_eq!(
            split_frontmatter("---\na: 1\n").unwrap_err().code,
            ProblemCode::ParseError
        );
    }

    #[test]
    fn empty_and_non_mapping() {
        assert!(parse_yaml("").unwrap().is_empty());
        assert!(parse_yaml("- a\n").is_err());
        assert!(parse_yaml("a: [\n").is_err());
    }
}

#[cfg(test)]
mod validate_tests {
    use super::*;
    use tempfile::TempDir;

    const MINIMAL: &str = "---\nschema: 1\nid: p/s\ntitle: T\nproject: p\nstatus: active\ncreated: 2026-09-12\nupdated: 2026-09-13\n---\nbody\n";

    fn doc(lines: &str) -> String {
        format!("---\n{lines}---\n")
    }

    fn with(base: &str, key: &str, line: &str) -> String {
        let lines: Vec<String> = base
            .lines()
            .filter(|l| *l != "---" && *l != "body" && !l.starts_with(&format!("{key}:")))
            .map(|l| format!("{l}\n"))
            .collect();
        doc(&format!("{}{line}\n", lines.concat()))
    }

    fn problems(content: &str) -> Vec<RecordProblem> {
        parse_record(content).unwrap_err()
    }

    #[test]
    fn minimal_record_parses() {
        let (record, body) = parse_record(MINIMAL).unwrap();
        assert_eq!(body, "body\n");
        assert_eq!(record.id, "p/s");
        assert_eq!(record.status, Status::Active);
        assert_eq!(record.stage, None);
        assert_eq!(record.priority, None);
        assert!(record.tags.is_empty());
        assert_eq!(record.status_changed, None);
        assert!(record.history.is_empty());
        assert_eq!(record.raw.len(), 7);
    }

    #[test]
    fn full_record_keeps_unknown_keys_and_history() {
        let content = "---\nschema: 1\nid: p/s\ntitle: T\nproject: p\nstatus: review\nstage: implement\npriority: 2\ntags: [a, b]\ncreated: 2026-09-12\nupdated: 2026-09-13\nstatus_changed: 2026-09-13\nowner: me\nhistory:\n  - {at: 2026-09-12, to: backlog}\n  - {at: 2026-09-13, to: review, note: ready}\n---\n";
        let (record, _) = parse_record(content).unwrap();
        assert_eq!(record.stage.as_deref(), Some("implement"));
        assert_eq!(record.priority, Some(2));
        assert_eq!(record.tags, ["a", "b"]);
        assert_eq!(record.status_changed.as_deref(), Some("2026-09-13"));
        assert_eq!(record.raw["owner"], "me");
        assert_eq!(record.history.len(), 2);
        assert_eq!(record.history[0].note, None);
        assert_eq!(record.history[1].to, Status::Review);
        assert_eq!(record.history[1].note.as_deref(), Some("ready"));
    }

    #[test]
    fn missing_frontmatter_is_single_parse_error() {
        let p = problems("no frontmatter\n");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].code, ProblemCode::ParseError);
    }

    #[test]
    fn multiple_missing_fields_reported_together() {
        let p = problems(&doc("schema: 1\nid: p/s\ntitle:\n"));
        let missing: Vec<&str> = p.iter().map(|p| p.message.as_str()).collect();
        assert_eq!(p.len(), 5, "{missing:?}");
        assert!(p.iter().all(|p| p.code == ProblemCode::MissingField));
        assert!(missing.contains(&"missing required field `title`"));
    }

    fn assert_invalid(content: &str, expected: &str) {
        let p = problems(content);
        assert_eq!(p.len(), 1, "{p:?}");
        assert_eq!(p[0].code, ProblemCode::InvalidValue);
        assert_eq!(p[0].message, expected);
    }

    #[test]
    fn bad_status() {
        assert_invalid(
            &with(MINIMAL, "status", "status: wip"),
            "invalid status `wip`; expected one of idea, backlog, active, review, done, dropped",
        );
    }

    #[test]
    fn bad_date() {
        assert_invalid(
            &with(MINIMAL, "created", "created: 2026-13-01"),
            "invalid date in `created`: `2026-13-01`",
        );
    }

    #[test]
    fn bad_schema() {
        assert_invalid(&with(MINIMAL, "schema", "schema: 2"), "`schema` must be 1");
    }

    #[test]
    fn bad_priority() {
        assert_invalid(
            &with(MINIMAL, "priority", "priority: high"),
            "`priority` must be an integer, got `high`",
        );
    }

    #[test]
    fn tags_not_a_list() {
        assert_invalid(
            &with(MINIMAL, "tags", "tags: nope"),
            "`tags` must be a list of strings",
        );
    }

    #[test]
    fn history_entry_with_bad_to() {
        let content = with(
            MINIMAL,
            "history",
            "history:\n  - {at: 2026-09-12, to: wip}",
        );
        let p = problems(&content);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].code, ProblemCode::InvalidValue);
        assert!(p[0].message.contains("history"));
    }

    #[test]
    fn history_entry_with_bad_date() {
        let content = with(
            MINIMAL,
            "history",
            "history:\n  - {at: 2026-02-30, to: idea}",
        );
        assert_eq!(problems(&content)[0].code, ProblemCode::InvalidValue);
    }

    #[test]
    fn stage_derivation() {
        let all = |input, proposal, design| Artifacts {
            input,
            proposal,
            design,
        };
        assert_eq!(
            derive_stage(None, all(true, true, false)).as_deref(),
            Some("proposal")
        );
        assert_eq!(
            derive_stage(Some("implement"), all(true, true, true)).as_deref(),
            Some("implement")
        );
        assert_eq!(derive_stage(None, all(false, false, false)), None);
    }

    #[test]
    fn artifacts_ignore_whitespace_only_and_missing() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("input.md"), "x\n").unwrap();
        std::fs::write(dir.path().join("proposal.md"), "y").unwrap();
        std::fs::write(dir.path().join("design.md"), " \n\t\n").unwrap();
        let found = artifacts(dir.path());
        assert!(found.input && found.proposal && !found.design);
        assert_eq!(derive_stage(None, found).as_deref(), Some("proposal"));
        assert_eq!(artifacts(&dir.path().join("absent")), Artifacts::default());
    }

    fn location(dir: &TempDir) -> TaskDir {
        TaskDir {
            id: "p/s".into(),
            project: "p".into(),
            slug: "s".into(),
            dir: dir.path().to_path_buf(),
        }
    }

    #[test]
    fn load_task_valid() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("task.md"), MINIMAL).unwrap();
        std::fs::write(dir.path().join("input.md"), "x").unwrap();
        let task = load_task(&location(&dir)).unwrap();
        assert_eq!(task.body, "body\n");
        assert_eq!(task.record.title, "T");
        assert_eq!(task.stage.as_deref(), Some("input"));
        assert!(task.artifacts.input);
    }

    #[test]
    fn load_task_broken_frontmatter() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("task.md"), "oops\n").unwrap();
        let err = load_task(&location(&dir)).unwrap_err();
        assert!(matches!(err, LedgerError::Invalid(_)));
        assert_eq!(err.exit_code(), 4);
        assert!(err.to_string().contains("task.md"));
    }

    #[test]
    fn load_task_missing_file_is_io() {
        let dir = TempDir::new().unwrap();
        assert_eq!(load_task(&location(&dir)).unwrap_err().exit_code(), 1);
    }
}
