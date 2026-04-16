use colored::Colorize;
use inquire::Select;

use crate::aider::AiderCommand;
use crate::config::Config;
use crate::error::{Result, VicraftError};
use crate::git;
use crate::tokens;

pub async fn run(staged: bool, cfg: &Config) -> Result<()> {
    git::assert_git_repo()?;

    let base = git::base_branch(&cfg.git.base_branch);
    let diff = if staged {
        git::diff_staged(&base)?
    } else {
        git::diff_base_to_head(&base)?
    };

    if diff.trim().is_empty() {
        let msg = if staged {
            "No changes to commit. Nothing staged — use 'git add' first."
        } else {
            "No changes to commit. No diff found vs base branch — did 'vicraft impl' create a WIP commit?"
        };
        return Err(VicraftError::validation(msg));
    }

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
        return Err(VicraftError::validation(
            "Aider returned an empty commit message.",
        ));
    }

    println!();
    println!("{}", "Proposed commit:".bold());
    println!("{}", "─".repeat(60));
    println!("{message}");
    println!("{}", "─".repeat(60));
    println!();

    let choice = Select::new("Action:", vec!["Accept", "Edit manually", "Cancel"])
        .prompt()
        .map_err(|e| VicraftError::validation(format!("prompt cancelled: {e}")))?;

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
        git::new_commit(message)?;
    } else {
        git::amend_commit(message)?;
    }
    Ok(())
}

fn edit_in_temp(initial: &str) -> Result<String> {
    use std::io::Write;

    let mut tmp = tempfile::NamedTempFile::new()
        .map_err(|e| VicraftError::validation(format!("failed to create temp file: {e}")))?;
    write!(tmp, "{initial}")
        .map_err(|e| VicraftError::validation(format!("failed to write temp file: {e}")))?;
    tmp.flush()
        .map_err(|e| VicraftError::validation(format!("failed to flush temp file: {e}")))?;

    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".into());

    let status = std::process::Command::new(&editor)
        .arg(tmp.path())
        .status()
        .map_err(|e| {
            VicraftError::external_tool(
                &editor,
                format!("failed to open editor: {e}"),
                "Set $EDITOR or $VISUAL to a valid editor command",
            )
        })?;
    if !status.success() {
        return Err(VicraftError::external_tool(
            &editor,
            "editor exited with non-zero status",
            "",
        ));
    }

    let edited = std::fs::read_to_string(tmp.path())
        .map_err(|e| VicraftError::validation(format!("failed to read edited file: {e}")))?;
    Ok(edited.trim().to_string())
}
