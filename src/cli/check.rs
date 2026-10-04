use crate::cli::output;
use crate::cli::{CheckArgs, Env};
use crate::core::check::{self, Problem};
use crate::core::error::Result;
use crate::core::scan;

pub fn run(args: &CheckArgs, json: bool, env: &Env) -> Result<u8> {
    let ledger = env.ledger()?;
    let problems = check::check(&ledger, args.fix)?;
    if json {
        output::print_json(&output::check_envelope(&problems));
    } else if problems.is_empty() {
        let checked = scan::task_dirs(&ledger.root)?.len();
        let noun = if checked == 1 { "task" } else { "tasks" };
        println!("ok: {checked} {noun} checked, no problems");
    } else {
        problems.iter().for_each(|p| println!("{}", line(p)));
    }
    Ok(u8::from(problems.iter().any(|p| !p.fixed)) * 4)
}

fn line(p: &Problem) -> String {
    let code = output::problem_out(p).code;
    let code = serde_json::to_value(code)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let fixed = if p.fixed { " (fixed)" } else { "" };
    format!("{}: {code} {}{fixed}", p.id, p.message)
}
