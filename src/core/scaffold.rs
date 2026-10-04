use std::path::{Path, PathBuf};

use crate::core::edit;
use crate::core::error::{LedgerError, Result};
use crate::core::lifecycle::Status;
use crate::core::record;
use crate::core::root::Ledger;
use crate::core::scan::{TaskDir, TASK_FILE};

const BUILTIN_TEMPLATES: [(&str, &str); 4] = [
    ("task.md", include_str!("templates/task.md")),
    ("input.md", include_str!("templates/input.md")),
    ("proposal.md", include_str!("templates/proposal.md")),
    ("design.md", include_str!("templates/design.md")),
];

pub struct NewTask {
    pub project: String,
    /// User slug, without date prefix.
    pub slug: String,
    pub title: Option<String>,
    pub status: Status,
    pub template: String,
    pub date_prefix: bool,
    pub today: String,
}

/// Non-empty, `[a-z0-9-]` only, not starting with `.` or `_`.
pub fn validate_name(name: &str) -> Result<()> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    valid.then_some(()).ok_or_else(|| {
        LedgerError::Invalid(format!(
            "invalid name '{name}': must be non-empty, contain only lowercase letters, digits and '-', and not start with '.' or '_'"
        ))
    })
}

/// Replace `{{key}}` placeholders by plain string substitution.
pub fn render(template: &str, vars: &[(&str, &str)]) -> String {
    vars.iter().fold(template.to_string(), |acc, (key, value)| {
        acc.replace(&format!("{{{{{key}}}}}"), value)
    })
}

/// `(file name, content)` for every `.md` in `<root>/<templates_dir>/<name>/`;
/// built-in templates when `name == "default"` and the dir is missing;
/// NotFound for another missing template.
pub fn template_files(ledger: &Ledger, name: &str) -> Result<Vec<(String, String)>> {
    let dir = ledger.root.join(&ledger.config.templates_dir).join(name);
    let files = if dir.is_dir() {
        read_template_dir(&dir)?
    } else if name == "default" {
        BUILTIN_TEMPLATES
            .iter()
            .map(|(file, content)| (file.to_string(), content.to_string()))
            .collect()
    } else {
        return Err(LedgerError::NotFound(format!(
            "template '{name}' not found at {}",
            dir.display()
        )));
    };
    files
        .iter()
        .any(|(file, _)| file == TASK_FILE)
        .then_some(files)
        .ok_or_else(|| {
            LedgerError::Invalid(format!(
                "template '{name}' has no {TASK_FILE} in {}",
                dir.display()
            ))
        })
}

fn read_template_dir(dir: &Path) -> Result<Vec<(String, String)>> {
    let io = |e| LedgerError::io(dir, e);
    let mut names = std::fs::read_dir(dir)
        .map_err(io)?
        .map(|entry| entry.map(|e| e.path()).map_err(io))
        .collect::<Result<Vec<PathBuf>>>()?
        .into_iter()
        .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "md"))
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_string))
        .collect::<Vec<_>>();
    names.sort();
    names
        .into_iter()
        .map(|file| {
            let path = dir.join(&file);
            std::fs::read_to_string(&path)
                .map(|content| (file, content))
                .map_err(|e| LedgerError::io(&path, e))
        })
        .collect()
}

/// Create the task dir (temp dir then rename), validate rendered `task.md`,
/// Conflict when the target exists, Invalid (and no leftovers) on a bad template.
pub fn create(ledger: &Ledger, new: &NewTask) -> Result<TaskDir> {
    validate_name(&new.project)?;
    validate_name(&new.slug)?;
    let slug = if new.date_prefix {
        format!("{}-{}", new.today, new.slug)
    } else {
        new.slug.clone()
    };
    let id = format!("{}/{slug}", new.project);
    let project_dir = ledger.root.join(&new.project);
    let dir = project_dir.join(&slug);
    if dir.exists() {
        return Err(LedgerError::Conflict(format!("{id} already exists")));
    }

    let title = new.title.clone().unwrap_or_else(|| new.slug.clone());
    let files = rendered_files(ledger, new, &id, &title)?;

    let project_created = !project_dir.exists();
    write_dir(&project_dir, &dir, &files).inspect_err(|_| {
        if project_created {
            let _ = std::fs::remove_dir(&project_dir);
        }
    })?;
    Ok(TaskDir {
        id,
        project: new.project.clone(),
        slug,
        dir,
    })
}

/// Rendered template files; `task.md` is rendered with a JSON-quoted title (a
/// valid YAML scalar) so titles containing `:` or `#` stay parseable.
fn rendered_files(
    ledger: &Ledger,
    new: &NewTask,
    id: &str,
    title: &str,
) -> Result<Vec<(String, String)>> {
    let status = new.status.as_str();
    let quoted = serde_json::to_string(title)
        .map_err(|e| LedgerError::Internal(format!("cannot encode title: {e}")))?;
    let vars = |title: &str| {
        [
            ("id", id.to_string()),
            ("title", title.to_string()),
            ("project", new.project.clone()),
            ("date", new.today.clone()),
            ("status", status.to_string()),
        ]
    };
    let render_with = |content: &str, title: &str| {
        let owned = vars(title);
        let pairs: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (*k, v.as_str())).collect();
        render(content, &pairs)
    };
    let files = template_files(ledger, &new.template)?
        .into_iter()
        .map(|(file, content)| {
            if file == TASK_FILE {
                edit::set_scalar(&render_with(&content, &quoted), "title", &quoted)
                    .map(|rendered| (file, rendered))
            } else {
                Ok((file, render_with(&content, title)))
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let task_md = files
        .iter()
        .find(|(file, _)| file == TASK_FILE)
        .map(|(_, content)| content.as_str())
        .unwrap_or_default();
    record::parse_record(task_md).map_err(|problems| {
        let messages = problems
            .iter()
            .map(|p| p.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        LedgerError::Invalid(format!(
            "template '{}' renders an invalid {TASK_FILE}: {messages}",
            new.template
        ))
    })?;
    Ok(files)
}

fn write_dir(project_dir: &Path, dir: &Path, files: &[(String, String)]) -> Result<()> {
    std::fs::create_dir_all(project_dir).map_err(|e| LedgerError::io(project_dir, e))?;
    let tmp = tempfile::Builder::new()
        .prefix(".new-")
        .tempdir_in(project_dir)
        .map_err(|e| LedgerError::io(project_dir, e))?;
    files.iter().try_for_each(|(file, content)| {
        let path = tmp.path().join(file);
        std::fs::write(&path, content).map_err(|e| LedgerError::io(&path, e))
    })?;
    std::fs::rename(tmp.path(), dir).map_err(|e| LedgerError::io(dir, e))?;
    let _ = tmp.keep();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::root::Config;

    const VARS: [(&str, &str); 5] = [
        ("id", "ingest/2026-10-04-x"),
        ("title", "X"),
        ("project", "ingest"),
        ("date", "2026-10-04"),
        ("status", "backlog"),
    ];

    fn ledger(root: &Path) -> Ledger {
        Ledger {
            root: root.to_path_buf(),
            config: Config::default(),
        }
    }

    fn new_task(slug: &str) -> NewTask {
        NewTask {
            project: "ingest".into(),
            slug: slug.into(),
            title: None,
            status: Status::Backlog,
            template: "default".into(),
            date_prefix: true,
            today: "2026-10-04".into(),
        }
    }

    #[test]
    fn builtin_task_template_parses() {
        let rendered = render(BUILTIN_TEMPLATES[0].1, &VARS);
        let (record, _) = record::parse_record(&rendered).expect("valid record");
        assert_eq!(record.id, "ingest/2026-10-04-x");
        assert_eq!(record.status, Status::Backlog);
        assert_eq!(record.history.len(), 1);
    }

    #[test]
    fn render_replaces_every_placeholder() {
        assert_eq!(
            render("{{a}}-{{a}}-{{b}}", &[("a", "1"), ("b", "2")]),
            "1-1-2"
        );
    }

    #[test]
    fn names_are_validated() {
        ["a", "a-1", "0"]
            .iter()
            .for_each(|n| assert!(validate_name(n).is_ok(), "{n}"));
        ["", "A", "a_b", ".a", "_a", "a b", "a/b"]
            .iter()
            .for_each(|n| assert!(validate_name(n).is_err(), "{n}"));
    }

    #[test]
    fn missing_named_template_is_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let err = template_files(&ledger(tmp.path()), "nope").unwrap_err();
        assert!(matches!(err, LedgerError::NotFound(_)));
    }

    #[test]
    fn default_template_falls_back_to_builtins() {
        let tmp = tempfile::tempdir().unwrap();
        let files = template_files(&ledger(tmp.path()), "default").unwrap();
        assert_eq!(files.len(), 4);
    }

    #[test]
    fn template_set_without_task_md_is_invalid() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("_templates/default");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("input.md"), "# {{title}}").unwrap();
        let err = template_files(&ledger(tmp.path()), "default").unwrap_err();
        assert!(matches!(err, LedgerError::Invalid(_)));
    }

    #[test]
    fn create_writes_task_and_conflicts_on_repeat() {
        let tmp = tempfile::tempdir().unwrap();
        let l = ledger(tmp.path());
        let task = create(&l, &new_task("x")).unwrap();
        assert_eq!(task.id, "ingest/2026-10-04-x");
        assert!(task.dir.join("task.md").is_file());
        let err = create(&l, &new_task("x")).unwrap_err();
        assert!(matches!(err, LedgerError::Conflict(_)));
    }

    #[test]
    fn failed_create_leaves_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("_templates/default");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("task.md"), "---\nschema: 1\n---\n").unwrap();
        let err = create(&ledger(tmp.path()), &new_task("x")).unwrap_err();
        assert!(matches!(err, LedgerError::Invalid(_)));
        assert!(!tmp.path().join("ingest").exists());
    }
}
