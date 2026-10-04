//! CLI layer: clap definitions, dispatch, and mapping of `LedgerError` to
//! stderr text, `{"error":...}` JSON on stdout (with `--json`) and exit codes.
//! Never prompts; all input comes from args, flags and env.

mod check;
mod ls;
mod new;
pub mod output;
mod schema;
mod show;
mod status;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::core::error::{LedgerError, Result};
use crate::core::lifecycle::Status;
use crate::core::root::{self, Ledger};

#[derive(Parser)]
#[command(
    name = "ledger",
    version,
    about = "CLI to manage spec driven artefacts for the task"
)]
pub struct Cli {
    /// Machine-readable JSON output (errors too)
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List and filter tasks
    Ls(LsArgs),
    /// Show one task
    Show(ShowArgs),
    /// Change a task's status
    #[command(alias = "mv")]
    Status(StatusArgs),
    /// Scaffold a new task from a template
    New(NewArgs),
    /// Validate all task records
    Check(CheckArgs),
    /// Print the JSON Schema of --json output
    Schema,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum SortKey {
    #[default]
    Id,
    Updated,
    Created,
    Status,
}

#[derive(Args)]
pub struct LsArgs {
    #[arg(long)]
    pub project: Option<String>,
    /// Comma-separated statuses; overrides the default done/dropped exclusion
    #[arg(long, value_delimiter = ',')]
    pub status: Option<Vec<Status>>,
    #[arg(long)]
    pub stage: Option<String>,
    #[arg(long)]
    pub tag: Option<String>,
    /// Include done and dropped tasks
    #[arg(long)]
    pub all: bool,
    #[arg(long, value_enum, default_value_t = SortKey::Id)]
    pub sort: SortKey,
    /// One JSON task object per line
    #[arg(long, conflicts_with_all = ["paths", "json"])]
    pub jsonl: bool,
    /// One absolute task directory per line
    #[arg(long, conflicts_with = "json")]
    pub paths: bool,
}

#[derive(Args)]
pub struct ShowArgs {
    /// Task id, path (e.g. `.`) or unique slug suffix
    pub task: String,
    /// Also print the task.md body
    #[arg(long)]
    pub body: bool,
}

#[derive(Args)]
pub struct StatusArgs {
    /// Task id, path (e.g. `.`) or unique slug suffix
    pub task: String,
    pub status: Status,
    /// Allow any transition
    #[arg(long)]
    pub force: bool,
    /// Allow done/dropped -> backlog
    #[arg(long)]
    pub reopen: bool,
    #[arg(long)]
    pub note: Option<String>,
    /// Validate and report without writing
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args)]
pub struct NewArgs {
    /// `<project>/<slug>`
    pub name: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long, value_enum, default_value_t = Status::Backlog)]
    pub status: Status,
    #[arg(long, default_value = "default")]
    pub template: String,
    #[arg(long)]
    pub no_date_prefix: bool,
}

#[derive(Args)]
pub struct CheckArgs {
    /// Repair id/project mismatches and a missing schema key
    #[arg(long)]
    pub fix: bool,
}

/// Process environment captured once at the boundary.
pub struct Env {
    pub cwd: PathBuf,
    pub env_root: Option<PathBuf>,
}

impl Env {
    fn from_process() -> Result<Env> {
        let cwd =
            std::env::current_dir().map_err(|e| LedgerError::io(std::path::Path::new("."), e))?;
        let env_root = std::env::var_os("LEDGER_ROOT")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from);
        Ok(Env { cwd, env_root })
    }

    pub fn ledger(&self) -> Result<Ledger> {
        root::discover(&self.cwd, self.env_root.as_deref())
    }
}

pub fn run() -> ExitCode {
    let cli = Cli::try_parse().unwrap_or_else(|e| e.exit());
    let json = cli.json;
    match Env::from_process().and_then(|env| dispatch(&cli.command, json, &env)) {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            output::print_error(&err, json);
            ExitCode::from(err.exit_code())
        }
    }
}

/// Returns the exit code for successful runs (`check` returns 4 on problems).
fn dispatch(command: &Command, json: bool, env: &Env) -> Result<u8> {
    match command {
        Command::Ls(args) => ls::run(args, json, env),
        Command::Show(args) => show::run(args, json, env),
        Command::Status(args) => status::run(args, json, env),
        Command::New(args) => new::run(args, json, env),
        Command::Check(args) => check::run(args, json, env),
        Command::Schema => schema::run(),
    }
}
