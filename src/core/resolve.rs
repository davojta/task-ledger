use std::path::Path;

use crate::core::error::{LedgerError, Result};
use crate::core::scan::{task_dirs, TaskDir};

/// Resolve a `<task>` reference, first matching step wins:
/// 1. exact task id; 2. filesystem path (absolute or relative to `cwd`, e.g. `.`)
///    to a task dir; 3. slug suffix: slug == reference or ends with `-<reference>`.
///
/// More than one match in the winning step = Ambiguous (candidates = ids);
/// no match = NotFound.
pub fn resolve(root: &Path, cwd: &Path, reference: &str) -> Result<TaskDir> {
    let tasks = task_dirs(root)?;
    let target = cwd.join(reference).canonicalize().ok();
    let suffix = format!("-{reference}");

    let by_id = |t: &TaskDir| t.id == reference;
    let by_path = |t: &TaskDir| target.is_some() && t.dir.canonicalize().ok() == target;
    let by_slug = |t: &TaskDir| t.slug == reference || t.slug.ends_with(&suffix);
    let steps: [&dyn Fn(&TaskDir) -> bool; 3] = [&by_id, &by_path, &by_slug];

    steps
        .iter()
        .map(|matches| tasks.iter().filter(|t| matches(t)).collect::<Vec<_>>())
        .find(|found| !found.is_empty())
        .map_or_else(
            || {
                Err(LedgerError::NotFound(format!(
                    "task '{reference}' not found"
                )))
            },
            |found| single(found, reference),
        )
}

/// `tasks` come sorted by id, so candidates are sorted too.
fn single(found: Vec<&TaskDir>, reference: &str) -> Result<TaskDir> {
    match found.as_slice() {
        [one] => Ok((*one).clone()),
        many => Err(LedgerError::Ambiguous {
            message: format!("task reference '{reference}' is ambiguous"),
            candidates: many.iter().map(|t| t.id.clone()).collect(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_file(path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::File::create(path)?;
        Ok(())
    }

    #[test]
    fn resolve_exact_id() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let root = tmpdir.path();

        create_file(&root.join("project1/2026-09-12-task-a/task.md")).unwrap();
        create_file(&root.join("project1/2026-09-12-task-b/task.md")).unwrap();

        let resolved = resolve(root, root, "project1/2026-09-12-task-a").unwrap();
        assert_eq!(resolved.id, "project1/2026-09-12-task-a");
    }

    #[test]
    fn resolve_relative_path_from_root() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let root = tmpdir.path();

        create_file(&root.join("ingest/2026-09-12-cdc-backfill/task.md")).unwrap();

        let resolved = resolve(root, root, "ingest/2026-09-12-cdc-backfill").unwrap();
        assert_eq!(resolved.id, "ingest/2026-09-12-cdc-backfill");
    }

    #[test]
    fn resolve_current_directory_dot() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let root = tmpdir.path();
        let task_dir = root.join("project/2026-09-12-task");

        create_file(&task_dir.join("task.md")).unwrap();

        let resolved = resolve(root, &task_dir, ".").unwrap();
        assert_eq!(resolved.id, "project/2026-09-12-task");
    }

    #[test]
    fn resolve_slug_suffix_full() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let root = tmpdir.path();

        create_file(&root.join("ingest/2026-09-12-cdc-backfill/task.md")).unwrap();

        let resolved = resolve(root, root, "2026-09-12-cdc-backfill").unwrap();
        assert_eq!(resolved.id, "ingest/2026-09-12-cdc-backfill");
    }

    #[test]
    fn resolve_slug_suffix_partial() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let root = tmpdir.path();

        create_file(&root.join("ingest/2026-09-12-cdc-backfill/task.md")).unwrap();

        let resolved = resolve(root, root, "cdc-backfill").unwrap();
        assert_eq!(resolved.id, "ingest/2026-09-12-cdc-backfill");
    }

    #[test]
    fn resolve_slug_suffix_requires_dash_boundary() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let root = tmpdir.path();

        create_file(&root.join("ingest/2026-09-12-cdc-backfill/task.md")).unwrap();

        let result = resolve(root, root, "ackfill");
        assert!(matches!(result, Err(LedgerError::NotFound(_))));
    }

    #[test]
    fn resolve_ambiguous_suffix_across_projects() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let root = tmpdir.path();

        create_file(&root.join("ingest/2026-01-01-ci-cache/task.md")).unwrap();
        create_file(&root.join("platform/2026-02-02-ci-cache/task.md")).unwrap();

        let result = resolve(root, root, "ci-cache");
        assert!(matches!(result, Err(LedgerError::Ambiguous { .. })));

        if let Err(LedgerError::Ambiguous { candidates, .. }) = result {
            let mut expected = vec!["ingest/2026-01-01-ci-cache", "platform/2026-02-02-ci-cache"];
            expected.sort();
            assert_eq!(candidates, expected);
        }
    }

    #[test]
    fn resolve_not_found() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let root = tmpdir.path();

        create_file(&root.join("project1/2026-09-12-task/task.md")).unwrap();

        let result = resolve(root, root, "nonexistent");
        assert!(matches!(result, Err(LedgerError::NotFound(_))));
    }

    #[test]
    fn resolve_exact_id_takes_precedence_over_suffix() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let root = tmpdir.path();

        create_file(&root.join("alpha/2026-09-12-exact/task.md")).unwrap();
        create_file(&root.join("beta/2026-01-01-exact-other/task.md")).unwrap();

        let resolved = resolve(root, root, "alpha/2026-09-12-exact").unwrap();
        assert_eq!(resolved.id, "alpha/2026-09-12-exact");
    }

    #[test]
    fn resolve_path_takes_precedence_over_suffix() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let root = tmpdir.path();

        create_file(&root.join("alpha/2026-09-12-target/task.md")).unwrap();
        create_file(&root.join("beta/2026-01-01-target-other/task.md")).unwrap();

        let alpha_dir = root.join("alpha");
        let resolved = resolve(root, &alpha_dir, "2026-09-12-target").unwrap();
        assert_eq!(resolved.id, "alpha/2026-09-12-target");
    }
}
