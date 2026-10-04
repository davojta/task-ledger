use crate::cli::{output, Env, NewArgs};
use crate::core::error::{LedgerError, Result};
use crate::core::scaffold::{self, NewTask};
use crate::core::{date, record};

pub fn run(args: &NewArgs, json: bool, env: &Env) -> Result<u8> {
    let ledger = env.ledger()?;
    let (project, slug) = split_name(&args.name)?;
    let new = NewTask {
        project: project.to_string(),
        slug: slug.to_string(),
        title: args.title.clone(),
        status: args.status,
        template: args.template.clone(),
        date_prefix: !args.no_date_prefix,
        today: date::today()?,
    };
    let location = scaffold::create(&ledger, &new)?;
    let task = record::load_task(&location)?;
    if json {
        output::print_json(&output::task_envelope(&task));
    } else {
        println!(
            "created {} at {}",
            task.location.id,
            task.location.dir.display()
        );
    }
    Ok(0)
}

fn split_name(name: &str) -> Result<(&str, &str)> {
    name.split_once('/')
        .filter(|(_, slug)| !slug.contains('/'))
        .ok_or_else(|| {
            LedgerError::Invalid(format!("invalid name '{name}': expected <project>/<slug>"))
        })
}
