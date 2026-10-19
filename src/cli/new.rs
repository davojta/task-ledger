use crate::cli::{output, Env, NewArgs};
use crate::core::error::{LedgerError, Result};
use crate::core::scaffold::{self, NewTask};
use crate::core::{date, record};

pub fn run(args: &NewArgs, json: bool, env: &Env) -> Result<u8> {
    let ledger = env.ledger()?;
    let (project, slug) = split_name(&args.name, ledger.config.project.as_deref())?;
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

/// `<project>/<slug>`; just `<slug>` in a flat ledger (`project` in ledger.toml).
fn split_name<'a>(name: &'a str, flat_project: Option<&'a str>) -> Result<(&'a str, &'a str)> {
    match flat_project {
        Some(project) => (!name.contains('/')).then_some((project, name)),
        None => name.split_once('/').filter(|(_, slug)| !slug.contains('/')),
    }
    .ok_or_else(|| {
        let expected = match flat_project {
            Some(project) => format!("<slug> (ledger.toml sets project = \"{project}\")"),
            None => "<project>/<slug>".to_string(),
        };
        LedgerError::Invalid(format!("invalid name '{name}': expected {expected}"))
    })
}
