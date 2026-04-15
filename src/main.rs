mod aider;
mod cli;
mod commands;
mod config;
mod git;
mod templates;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = config::load()?;

    match cli.command {
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
