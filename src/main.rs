use clap::{Parser, Subcommand};
use task_ledger::{create_greeting, process_input};

#[derive(Parser)]
#[command(
    name = "task-ledger",
    version = "0.1.0",
    about = "CLI to manage spec driven artefacts for the task"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Say hello to the world or to a specific person
    Hello {
        /// Name to include in the greeting
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Process input text and return a formatted response
    Process {
        /// Input text to process
        input: Option<String>,
        /// Convert output to uppercase
        #[arg(short, long)]
        uppercase: bool,
    },
    /// Show version information
    Version,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Hello { name } => {
            println!("{}", create_greeting(name.as_deref()));
        }
        Commands::Process { input, uppercase } => {
            let text = input.unwrap_or_default();
            let result = process_input(&text);
            if uppercase {
                println!("{}", result.to_uppercase());
            } else {
                println!("{}", result);
            }
        }
        Commands::Version => {
            println!("task-ledger version 0.1.0");
        }
    }
}
