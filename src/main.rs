mod aider;
mod cli;
mod commands;
mod config;
mod error;
mod git;
mod templates;
mod tokens;

use clap::Parser;
use cli::{Cli, Command};
use error::VicraftError;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = run(cli.command).await;

    if let Err(err) = result {
        let command_name = std::env::args().nth(1).unwrap_or_default();
        err.log_error_to_file(&command_name);
        err.format_error(cli.verbose);
        std::process::exit(err.exit_code());
    }
}

async fn run(command: Command) -> Result<(), VicraftError> {
    let cfg = config::load()?;

    match command {
        Command::Init => commands::init::run(),
        Command::Scan => commands::scan::run(&cfg).await,
        Command::NewIssue { name, open } => commands::new_issue::run(&name, open),
        Command::Spec { input } => commands::spec::run(&input, &cfg).await,
        Command::Plan { input } => commands::plan::run(&input, &cfg).await,
        Command::Impl { plan } => commands::implement::run(&plan, &cfg).await,
        Command::Review => commands::review::run(&cfg).await,
        Command::Commit { staged } => commands::commit::run(staged, &cfg).await,
        Command::Pr => commands::pr::run(&cfg).await,
        Command::ClearContext => commands::clear_context::run(),
        Command::Skills { action } => commands::skills::run(action),
    }
}
