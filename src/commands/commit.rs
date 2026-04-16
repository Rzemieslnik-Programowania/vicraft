use anyhow::{bail, Result};
use colored::Colorize;
use inquire::Select;

use crate::aider::AiderCommand;
use crate::config::Config;
use crate::git;
use crate::tokens;

pub async fn run(staged: bool, cfg: &Config) -> Result<()> {
    git::assert_git_repo()?;

    // 1. Get diff to analyze
    let base = git::base_branch(&cfg.git.base_branch);
    let diff = if staged {
        git::diff_staged(&base)?
    } else {
        git::diff_base_to_head(&base)?
    };

    if diff.trim().is_empty() {
        bail!(
            "No changes to commit.\n\
             {}",
            if staged {
                "Nothing staged. Use `git add` first."
            } else {
                "No diff found vs base branch. Did `vicraft impl` create a WIP commit?"
            }
        );
    }

    // 2. Generate conventional commit message
    let model = cfg.model_for_step("commit");
    println!("{}", "Generating commit message...".bold());
    println!("  Model: {}", model.cyan());
    let prompt = format!(
        r#"Generate a conventional commit message for the following diff.

Rules:
- Format: <type>(<scope>): <subject>
- Followed by a blank line and bullet points describing key changes
- Type: feat | fix | refactor | test | docs | chore | ci
- Scope: the main module/area changed (optional but preferred)
- Subject: imperative mood, lowercase, no period, max 72 chars
- Bullet points: concise, start with a verb, max 5 bullets
- Output ONLY the commit message — no commentary, no backticks

## Diff
{diff}
"#
    );

    let result = AiderCommand::ask(&cfg.aider, &prompt)
        .override_model(model)
        .run_capture()?;
    tokens::display_usage(&result.usage);
    let message = result.stdout.trim().to_string();

    if message.is_empty() {
        bail!("Aider returned an empty commit message.");
    }

    // 3. Show proposal and ask for confirmation
    println!();
    println!("{}", "Proposed commit:".bold());
    println!("{}", "─".repeat(60));
    println!("{message}");
    println!("{}", "─".repeat(60));
    println!();

    let choice = Select::new("Action:", vec!["Accept", "Edit manually", "Cancel"]).prompt()?;

    match choice {
        "Accept" => apply_commit(staged, &message)?,
        "Edit manually" => {
            let edited = edit_in_temp(&message)?;
            apply_commit(staged, &edited)?;
        }
        _ => {
            println!("{}", "Cancelled.".yellow());
            return Ok(());
        }
    }

    println!("{} Committed.", "✓".green());
    println!();
    println!("Create a PR when ready:");
    println!("   {}", "vicraft pr".cyan());

    Ok(())
}

fn apply_commit(staged: bool, message: &str) -> Result<()> {
    if staged {
        // Staged mode: create a new commit from staged files
        git::new_commit(message)?;
    } else {
        // Normal mode: amend the WIP commit
        git::amend_commit(message)?;
    }
    Ok(())
}

fn edit_in_temp(initial: &str) -> Result<String> {
    use std::io::Write;

    let mut tmp = tempfile::NamedTempFile::new()?;
    write!(tmp, "{initial}")?;
    tmp.flush()?;

    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".into());

    std::process::Command::new(&editor)
        .arg(tmp.path())
        .status()?;

    let edited = std::fs::read_to_string(tmp.path())?;
    Ok(edited.trim().to_string())
}
