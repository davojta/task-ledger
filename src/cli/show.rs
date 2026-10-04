use crate::cli::output::{print_json, task_envelope};
use crate::cli::{Env, ShowArgs};
use crate::core::error::Result;
use crate::core::record::{load_task, Task};
use crate::core::resolve::resolve;

fn yes_no(present: bool) -> &'static str {
    if present {
        "yes"
    } else {
        "no"
    }
}

fn render(task: &Task) -> String {
    let r = &task.record;
    let a = task.artifacts;
    let dash = |v: Option<String>| v.unwrap_or_else(|| "-".to_string());
    [
        format!("id: {}", task.location.id),
        format!("title: {}", r.title),
        format!("project: {}", task.location.project),
        format!("status: {}", r.status.as_str()),
        format!("stage: {}", dash(task.stage.clone())),
        format!("priority: {}", dash(r.priority.map(|p| p.to_string()))),
        format!("tags: {}", r.tags.join(", ")),
        format!("created: {}", r.created),
        format!("updated: {}", r.updated),
        format!("status_changed: {}", dash(r.status_changed.clone())),
        format!("path: {}", task.location.dir.display()),
        format!(
            "artifacts: input {} proposal {} design {}",
            yes_no(a.input),
            yes_no(a.proposal),
            yes_no(a.design)
        ),
    ]
    .join("\n")
}

pub fn run(args: &ShowArgs, json: bool, env: &Env) -> Result<u8> {
    let ledger = env.ledger()?;
    let dir = resolve(&ledger.root, &env.cwd, &args.task)?;
    let task = load_task(&dir)?;
    if json {
        print_json(&task_envelope(&task));
    } else {
        println!("{}", render(&task));
        if args.body {
            print!("\n{}", task.body);
        }
    }
    Ok(0)
}
