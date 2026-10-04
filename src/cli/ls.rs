use std::cmp::Ordering;

use crate::cli::output::{list_envelope, print_json, task_out};
use crate::cli::{Env, LsArgs, SortKey};
use crate::core::error::{LedgerError, Result};
use crate::core::record::{load_task, Task};
use crate::core::scan::task_dirs;

fn matches(args: &LsArgs, task: &Task) -> bool {
    let status_ok = match &args.status {
        Some(statuses) => statuses.contains(&task.record.status),
        None => args.all || !task.record.status.is_terminal(),
    };
    status_ok
        && args
            .project
            .as_ref()
            .is_none_or(|p| *p == task.location.project)
        && args
            .stage
            .as_ref()
            .is_none_or(|s| task.stage.as_ref() == Some(s))
        && args
            .tag
            .as_ref()
            .is_none_or(|t| task.record.tags.contains(t))
}

fn compare(key: SortKey, a: &Task, b: &Task) -> Ordering {
    let primary = match key {
        SortKey::Id => Ordering::Equal,
        SortKey::Updated => b.record.updated.cmp(&a.record.updated),
        SortKey::Created => b.record.created.cmp(&a.record.created),
        SortKey::Status => a.record.status.cmp(&b.record.status),
    };
    primary.then_with(|| a.location.id.cmp(&b.location.id))
}

fn table(tasks: &[Task]) -> String {
    let header = ["ID", "STATUS", "STAGE", "UPDATED", "TITLE"].map(String::from);
    let rows = std::iter::once(header).chain(tasks.iter().map(|t| {
        [
            t.location.id.clone(),
            t.record.status.as_str().to_string(),
            t.stage.clone().unwrap_or_else(|| "-".to_string()),
            t.record.updated.clone(),
            t.record.title.clone(),
        ]
    }));
    let rows: Vec<_> = rows.collect();
    let widths: Vec<usize> = (0..4)
        .map(|i| rows.iter().map(|r| r[i].chars().count()).max().unwrap_or(0))
        .collect();
    rows.iter()
        .map(|r| {
            let cells: Vec<String> = (0..4)
                .map(|i| format!("{:<w$}", r[i], w = widths[i]))
                .collect();
            format!("{}  {}", cells.join("  "), r[4])
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn run(args: &LsArgs, json: bool, env: &Env) -> Result<u8> {
    let ledger = env.ledger()?;
    let mut tasks: Vec<Task> = task_dirs(&ledger.root)?
        .iter()
        .filter_map(|dir| match load_task(dir) {
            Ok(task) => Some(task),
            Err(err) => {
                eprintln!("warning: {err}");
                None
            }
        })
        .filter(|task| matches(args, task))
        .collect();
    tasks.sort_by(|a, b| compare(args.sort, a, b));

    if json {
        print_json(&list_envelope(&tasks));
    } else if args.jsonl {
        tasks.iter().for_each(|t| print_json(&task_out(t)));
    } else if args.paths {
        for task in &tasks {
            let path = std::path::absolute(&task.location.dir)
                .map_err(|e| LedgerError::io(&task.location.dir, e))?;
            println!("{}", path.display());
        }
    } else {
        println!("{}", table(&tasks));
    }
    Ok(0)
}
