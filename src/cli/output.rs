use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::core::check::Problem;
use crate::core::error::LedgerError;
use crate::core::lifecycle::Status;
use crate::core::record::{Artifacts, HistoryEntry, ProblemCode, Task, KNOWN_KEYS};

pub const SCHEMA_VERSION: u32 = 1;

/// A task as exposed in JSON: known fields (id/project/stage are the reported,
/// path-derived or derived values), `path`, `artifacts`, plus every unknown
/// frontmatter key flattened in.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TaskOut {
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
    pub path: String,
    pub artifacts: Artifacts,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ListEnvelope {
    pub schema_version: u32,
    pub tasks: Vec<TaskOut>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TaskEnvelope {
    pub schema_version: u32,
    pub task: TaskOut,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ProblemOut {
    pub path: String,
    pub id: String,
    pub code: ProblemCode,
    pub message: String,
    pub fixed: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CheckEnvelope {
    pub schema_version: u32,
    pub problems: Vec<ProblemOut>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

pub fn task_out(task: &Task) -> TaskOut {
    let r = &task.record;
    let extra = r
        .raw
        .iter()
        .filter(|(k, _)| !KNOWN_KEYS.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    TaskOut {
        schema: r.schema,
        id: task.location.id.clone(),
        title: r.title.clone(),
        project: task.location.project.clone(),
        status: r.status,
        stage: task.stage.clone(),
        priority: r.priority,
        tags: r.tags.clone(),
        created: r.created.clone(),
        updated: r.updated.clone(),
        status_changed: r.status_changed.clone(),
        history: r.history.clone(),
        path: task.location.dir.display().to_string(),
        artifacts: task.artifacts,
        extra,
    }
}

pub fn problem_out(p: &Problem) -> ProblemOut {
    ProblemOut {
        path: p.path.display().to_string(),
        id: p.id.clone(),
        code: p.code,
        message: p.message.clone(),
        fixed: p.fixed,
    }
}

pub fn list_envelope(tasks: &[Task]) -> ListEnvelope {
    ListEnvelope {
        schema_version: SCHEMA_VERSION,
        tasks: tasks.iter().map(task_out).collect(),
    }
}

pub fn task_envelope(task: &Task) -> TaskEnvelope {
    TaskEnvelope {
        schema_version: SCHEMA_VERSION,
        task: task_out(task),
    }
}

pub fn check_envelope(problems: &[Problem]) -> CheckEnvelope {
    CheckEnvelope {
        schema_version: SCHEMA_VERSION,
        problems: problems.iter().map(problem_out).collect(),
    }
}

pub fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string(value) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("error: failed to serialize output: {e}"),
    }
}

pub fn print_error(err: &LedgerError, json: bool) {
    eprintln!("error: {err}");
    if json {
        print_json(&ErrorEnvelope {
            error: ErrorBody {
                code: err.json_code().to_string(),
                message: err.to_string(),
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::record::Record;
    use crate::core::scan::TaskDir;
    use std::path::PathBuf;

    fn sample_task() -> Task {
        let mut raw = Map::new();
        raw.insert("status".into(), Value::from("active"));
        raw.insert("owner".into(), Value::from("me"));
        Task {
            location: TaskDir {
                id: "ingest/2026-09-12-cdc".into(),
                project: "ingest".into(),
                slug: "2026-09-12-cdc".into(),
                dir: PathBuf::from("/vault/ingest/2026-09-12-cdc"),
            },
            record: Record {
                schema: 1,
                id: "ingest/old".into(),
                title: "CDC".into(),
                project: "ingest".into(),
                status: Status::Active,
                stage: None,
                priority: None,
                tags: vec![],
                created: "2026-09-12".into(),
                updated: "2026-09-12".into(),
                status_changed: None,
                history: vec![],
                raw,
            },
            artifacts: Artifacts::default(),
            stage: None,
            body: String::new(),
        }
    }

    #[test]
    fn task_envelope_has_version_extras_and_path_derived_id() {
        let v = serde_json::to_value(task_envelope(&sample_task())).unwrap();
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["task"]["owner"], "me");
        assert_eq!(v["task"]["id"], "ingest/2026-09-12-cdc");
        assert_eq!(v["task"]["status"], "active");
        assert_eq!(v["task"]["artifacts"]["input"], false);
    }
}
