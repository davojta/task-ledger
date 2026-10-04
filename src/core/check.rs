use std::path::PathBuf;

use crate::core::edit;
use crate::core::error::{LedgerError, Result};
use crate::core::record::{
    missing_field_message, parse_yaml, split_frontmatter, validate, ProblemCode, RecordProblem,
};
use crate::core::root::Ledger;
use crate::core::scan::{self, TaskDir};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub path: PathBuf,
    pub id: String,
    pub code: ProblemCode,
    pub message: String,
    pub fixed: bool,
}

/// Validate every task and orphan under the root. With `fix`, repair only
/// `IdMismatch`, `ProjectMismatch` and a missing `schema` key (surgical,
/// atomic) and mark them `fixed`.
pub fn check(ledger: &Ledger, fix: bool) -> Result<Vec<Problem>> {
    let mut problems = scan::task_dirs(&ledger.root)?
        .iter()
        .map(|task| check_task(task, fix))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .chain(scan::orphan_dirs(&ledger.root)?.iter().map(orphan_problem))
        .collect::<Vec<_>>();
    problems.sort_by(|a, b| (&a.id, a.code).cmp(&(&b.id, b.code)));
    Ok(problems)
}

fn problem(task: &TaskDir, code: ProblemCode, message: String) -> Problem {
    Problem {
        path: task.dir.clone(),
        id: task.id.clone(),
        code,
        message,
        fixed: false,
    }
}

fn orphan_problem(task: &TaskDir) -> Problem {
    problem(
        task,
        ProblemCode::Orphan,
        "task directory has artifacts but no task.md".into(),
    )
}

fn from_record(task: &TaskDir, p: RecordProblem) -> Problem {
    problem(task, p.code, p.message)
}

fn mismatch(
    task: &TaskDir,
    map: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    expected: &str,
    code: ProblemCode,
) -> Option<Problem> {
    map.get(key)
        .and_then(|v| v.as_str())
        .filter(|actual| *actual != expected)
        .map(|actual| {
            problem(
                task,
                code,
                format!("{key} `{actual}` does not match location `{expected}`"),
            )
        })
}

fn problems_for(content: &str, task: &TaskDir) -> Vec<Problem> {
    let map = match split_frontmatter(content).and_then(|(yaml, _)| parse_yaml(&yaml)) {
        Ok(map) => map,
        Err(p) => return vec![from_record(task, p)],
    };
    let mismatches = [
        mismatch(task, &map, "id", &task.id, ProblemCode::IdMismatch),
        mismatch(
            task,
            &map,
            "project",
            &task.project,
            ProblemCode::ProjectMismatch,
        ),
    ]
    .into_iter()
    .flatten();
    let invalid = validate(map).err().unwrap_or_default();
    invalid
        .into_iter()
        .map(|p| from_record(task, p))
        .chain(mismatches)
        .collect()
}

/// The `(key, value)` frontmatter edit that repairs `problem`, if it is fixable.
fn fix_for(problem: &Problem, task: &TaskDir) -> Option<(&'static str, String)> {
    match problem.code {
        ProblemCode::IdMismatch => Some(("id", task.id.clone())),
        ProblemCode::ProjectMismatch => Some(("project", task.project.clone())),
        ProblemCode::MissingField if problem.message == missing_field_message("schema") => {
            Some(("schema", "1".into()))
        }
        _ => None,
    }
}

/// Apply every fixable problem in one pipeline; edits that fail are skipped.
fn apply_fixes(content: &str, problems: &mut [Problem], task: &TaskDir) -> Option<String> {
    let fixed = problems.iter_mut().fold(content.to_string(), |acc, p| {
        match fix_for(p, task).map(|(key, value)| edit::set_scalar(&acc, key, &value)) {
            Some(Ok(next)) => {
                p.fixed = true;
                next
            }
            _ => acc,
        }
    });
    (fixed != content).then_some(fixed)
}

fn check_task(task: &TaskDir, fix: bool) -> Result<Vec<Problem>> {
    let path = task.task_file();
    let content = std::fs::read_to_string(&path).map_err(|e| LedgerError::io(&path, e))?;
    let mut problems = problems_for(&content, task);
    if fix {
        if let Some(fixed) = apply_fixes(&content, &mut problems, task) {
            edit::write_atomic(&path, &fixed)?;
        }
    }
    Ok(problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "ingest/2026-09-12-cdc";

    fn task() -> TaskDir {
        TaskDir {
            id: ID.into(),
            project: "ingest".into(),
            slug: "2026-09-12-cdc".into(),
            dir: PathBuf::from("/r/ingest/2026-09-12-cdc"),
        }
    }

    fn doc(id: &str, project: &str, status: &str) -> String {
        format!("---\nschema: 1\nid: {id}\ntitle: T\nproject: {project}\nstatus: {status}\ncreated: 2026-09-01\nupdated: 2026-09-01\n---\nbody\n")
    }

    fn codes(content: &str) -> Vec<ProblemCode> {
        problems_for(content, &task())
            .iter()
            .map(|p| p.code)
            .collect()
    }

    #[test]
    fn clean_record_has_no_problems() {
        assert!(codes(&doc(ID, "ingest", "active")).is_empty());
    }

    #[test]
    fn id_and_project_mismatch_messages() {
        let problems = problems_for(&doc("x/y", "other", "active"), &task());
        let messages: Vec<_> = problems.iter().map(|p| p.message.as_str()).collect();
        assert_eq!(
            messages,
            [
                format!("id `x/y` does not match location `{ID}`"),
                "project `other` does not match location `ingest`".to_string()
            ]
        );
    }

    #[test]
    fn mismatch_reported_alongside_invalid_value() {
        assert_eq!(
            codes(&doc("x/y", "ingest", "wip")),
            [ProblemCode::InvalidValue, ProblemCode::IdMismatch]
        );
    }

    #[test]
    fn missing_frontmatter_is_parse_error() {
        assert_eq!(codes("no frontmatter\n"), [ProblemCode::ParseError]);
    }

    #[test]
    fn missing_schema_is_missing_field() {
        let content = doc(ID, "ingest", "active").replace("schema: 1\n", "");
        assert_eq!(codes(&content), [ProblemCode::MissingField]);
    }

    #[test]
    fn apply_fixes_is_surgical_and_skips_unfixable() {
        let content = doc("x/y", "ingest", "wip").replace("schema: 1\n", "");
        let mut problems = problems_for(&content, &task());
        let fixed = apply_fixes(&content, &mut problems, &task()).unwrap();
        assert_eq!(
            fixed,
            format!("---\nid: {ID}\ntitle: T\nproject: ingest\nstatus: wip\ncreated: 2026-09-01\nupdated: 2026-09-01\nschema: 1\n---\nbody\n")
        );
        let flags: Vec<_> = problems.iter().map(|p| (p.code, p.fixed)).collect();
        assert_eq!(
            flags,
            [
                (ProblemCode::MissingField, true),
                (ProblemCode::InvalidValue, false),
                (ProblemCode::IdMismatch, true)
            ]
        );
    }

    #[test]
    fn failed_edit_leaves_problem_unfixed() {
        let content = "---\nschema: 1\nid:\n  - x\ntitle: T\nproject: ingest\nstatus: active\ncreated: 2026-09-01\nupdated: 2026-09-01\n---\n";
        let mut problems = problems_for(content, &task());
        assert!(apply_fixes(content, &mut problems, &task()).is_none());
        assert!(problems.iter().all(|p| !p.fixed));
    }
}
