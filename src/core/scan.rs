use std::path::{Path, PathBuf};

use crate::core::error::{LedgerError, Result};
use crate::core::root::Ledger;

pub const TASK_FILE: &str = "task.md";
pub const ARTIFACT_NAMES: [&str; 3] = ["input", "proposal", "design"];

/// A task directory: `<root>/<project>/<slug>/`, or `<root>/<slug>/` in a
/// flat ledger (`project` set in `ledger.toml`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDir {
    /// `<project>/<slug>`, or `<slug>` in a flat ledger
    pub id: String,
    pub project: String,
    pub slug: String,
    pub dir: PathBuf,
}

impl TaskDir {
    pub fn task_file(&self) -> PathBuf {
        self.dir.join(TASK_FILE)
    }

    fn has_task_file(&self) -> bool {
        self.task_file().is_file()
    }

    fn has_artifact(&self) -> bool {
        ARTIFACT_NAMES
            .iter()
            .any(|name| self.dir.join(format!("{name}.md")).is_file())
    }
}

/// True for directory names the scanner skips (`.`- or `_`-prefixed).
pub fn is_ignored(name: &str) -> bool {
    name.starts_with('.') || name.starts_with('_')
}

/// `(name, path)` of non-ignored, UTF-8-named subdirectories of `dir`.
fn subdirs(dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| LedgerError::io(dir, e))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|e| LedgerError::io(dir, e))?;
    Ok(entries
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?.to_string();
            (!is_ignored(&name)).then_some((name, path))
        })
        .collect())
}

/// Every candidate task directory, sorted by id.
fn candidate_dirs(ledger: &Ledger) -> Result<Vec<TaskDir>> {
    let projects = match &ledger.config.project {
        Some(project) => vec![(project.clone(), ledger.root.clone())],
        None => subdirs(&ledger.root)?,
    };
    let mut dirs = projects
        .into_iter()
        .map(|(project, project_path)| {
            subdirs(&project_path).map(|slugs| {
                slugs
                    .into_iter()
                    .map(|(slug, dir)| TaskDir {
                        id: ledger.task_id(&project, &slug),
                        project: project.clone(),
                        slug,
                        dir,
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    dirs.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(dirs)
}

/// All task dirs (ignored names skipped) that contain `task.md`, sorted by id.
pub fn task_dirs(ledger: &Ledger) -> Result<Vec<TaskDir>> {
    Ok(candidate_dirs(ledger)?
        .into_iter()
        .filter(TaskDir::has_task_file)
        .collect())
}

/// Task dirs without `task.md` but with any of
/// `input.md`/`proposal.md`/`design.md`, sorted by id.
pub fn orphan_dirs(ledger: &Ledger) -> Result<Vec<TaskDir>> {
    Ok(candidate_dirs(ledger)?
        .into_iter()
        .filter(|d| !d.has_task_file() && d.has_artifact())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::root::Config;
    use std::fs;

    fn ledger(root: &Path, project: Option<&str>) -> Ledger {
        Ledger {
            root: root.to_path_buf(),
            config: Config {
                project: project.map(str::to_string),
                ..Config::default()
            },
        }
    }

    fn create_file(path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::File::create(path)?;
        Ok(())
    }

    #[test]
    fn scan_task_and_orphan_dirs() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let root = tmpdir.path();

        create_file(&root.join("_templates/default/task.md")).unwrap();
        create_file(&root.join(".git/x/task.md")).unwrap();
        create_file(&root.join("ingest/_draft/task.md")).unwrap();
        create_file(&root.join("ingest/2026-09-12-cdc/task.md")).unwrap();
        create_file(&root.join("platform/2026-08-03-ci/input.md")).unwrap();
        create_file(&root.join("platform/notes.md")).unwrap();
        fs::create_dir_all(root.join("platform/empty")).unwrap();

        let tasks = task_dirs(&ledger(root, None)).unwrap();
        let task_ids: Vec<String> = tasks.iter().map(|t| t.id.clone()).collect();
        assert_eq!(task_ids, vec!["ingest/2026-09-12-cdc"]);

        let orphans = orphan_dirs(&ledger(root, None)).unwrap();
        let orphan_ids: Vec<String> = orphans.iter().map(|o| o.id.clone()).collect();
        assert_eq!(orphan_ids, vec!["platform/2026-08-03-ci"]);
    }

    #[test]
    fn scan_sorts_multiple_tasks() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let root = tmpdir.path();

        create_file(&root.join("alpha/2026-09-12-b/task.md")).unwrap();
        create_file(&root.join("alpha/2026-09-12-a/task.md")).unwrap();
        create_file(&root.join("beta/2026-01-01-z/task.md")).unwrap();

        let tasks = task_dirs(&ledger(root, None)).unwrap();
        let task_ids: Vec<String> = tasks.iter().map(|t| t.id.clone()).collect();
        assert_eq!(
            task_ids,
            vec![
                "alpha/2026-09-12-a",
                "alpha/2026-09-12-b",
                "beta/2026-01-01-z"
            ]
        );
    }

    #[test]
    fn orphan_dirs_checks_all_artifact_names() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let root = tmpdir.path();

        create_file(&root.join("p1/proposal/proposal.md")).unwrap();
        create_file(&root.join("p2/design/design.md")).unwrap();
        create_file(&root.join("p3/input/input.md")).unwrap();
        create_file(&root.join("p4/multiple/proposal.md")).unwrap();
        create_file(&root.join("p4/multiple/design.md")).unwrap();

        let orphans = orphan_dirs(&ledger(root, None)).unwrap();
        let orphan_ids: Vec<String> = orphans.iter().map(|o| o.id.clone()).collect();
        assert_eq!(
            orphan_ids,
            vec!["p1/proposal", "p2/design", "p3/input", "p4/multiple"]
        );
    }

    #[test]
    fn flat_ledger_scans_root_level_task_dirs() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let root = tmpdir.path();

        create_file(&root.join("_templates/default/task.md")).unwrap();
        create_file(&root.join("2026-10-19-plan/task.md")).unwrap();
        create_file(&root.join("2026-10-20-draft/proposal.md")).unwrap();
        create_file(&root.join("nested/2026-10-21-x/task.md")).unwrap();

        let flat = ledger(root, Some("anki-helpers"));
        let tasks = task_dirs(&flat).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "2026-10-19-plan");
        assert_eq!(tasks[0].project, "anki-helpers");
        assert_eq!(tasks[0].dir, root.join("2026-10-19-plan"));

        let orphans = orphan_dirs(&flat).unwrap();
        let orphan_ids: Vec<String> = orphans.iter().map(|o| o.id.clone()).collect();
        assert_eq!(orphan_ids, vec!["2026-10-20-draft"]);
    }
}
