use crate::cli::output;
use crate::cli::{Env, StatusArgs};
use crate::core::error::{LedgerError, Result};
use crate::core::lifecycle::{self, Status, TransitionOpts};
use crate::core::record::{self, HistoryEntry, Task};
use crate::core::{date, edit, resolve};

pub fn run(args: &StatusArgs, json: bool, env: &Env) -> Result<u8> {
    let ledger = env.ledger()?;
    let dir = resolve::resolve(&ledger.root, &env.cwd, &args.task)?;
    let task = record::load_task(&dir)?;
    let from = task.record.status;

    if from == args.status {
        report(&task, from, args, json, true);
        return Ok(0);
    }

    let opts = TransitionOpts {
        reopen: args.reopen,
        force: args.force,
    };
    lifecycle::check_transition(from, args.status, opts)?;

    let task_file = dir.task_file();
    let content =
        std::fs::read_to_string(&task_file).map_err(|e| LedgerError::io(&task_file, e))?;
    let new_content = apply_transition(&content, args, &date::today()?)?;
    let new_task = reparse(&task, &new_content)?;

    if !args.dry_run {
        edit::write_atomic(&task_file, &new_content)?;
    }
    report(&new_task, from, args, json, false);
    Ok(0)
}

fn apply_transition(content: &str, args: &StatusArgs, today: &str) -> Result<String> {
    let to = args.status;
    let content = edit::set_scalar(content, "status", to.as_str())?;
    let content = edit::set_scalar(&content, "updated", today)?;
    let content = edit::set_scalar(&content, "status_changed", today)?;
    edit::append_history(
        &content,
        &HistoryEntry {
            at: today.to_string(),
            to,
            note: args.note.clone(),
        },
    )
}

fn reparse(task: &Task, content: &str) -> Result<Task> {
    let (record, body) = record::parse_record(content).map_err(|problems| {
        let messages = problems
            .iter()
            .map(|p| p.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        LedgerError::Invalid(format!("edit produced invalid task.md: {messages}"))
    })?;
    let stage = record::derive_stage(record.stage.as_deref(), task.artifacts);
    Ok(Task {
        location: task.location.clone(),
        record,
        artifacts: task.artifacts,
        stage,
        body,
    })
}

fn report(task: &Task, from: Status, args: &StatusArgs, json: bool, unchanged: bool) {
    if json {
        output::print_json(&output::task_envelope(task));
    } else if unchanged {
        println!("{}: already {from}", task.location.id);
    } else {
        let suffix = if args.dry_run { " (dry run)" } else { "" };
        println!("{}: {from} -> {}{suffix}", task.location.id, args.status);
    }
}
