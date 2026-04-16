use colored::Colorize;
use inquire::Select;
use std::process::Command;

use crate::aider::AiderCommand;
use crate::config::Config;
use crate::error::{Result, VicraftError};
use crate::git;
use crate::tokens;

pub async fn run(cfg: &Config) -> Result<()> {
    git::assert_git_repo()?;

    if !gh_available() {
        return Err(VicraftError::external_tool(
            "gh",
            "GitHub CLI (gh) not found",
            "Install with: sudo dnf install gh && gh auth login",
        ));
    }

    let branch = git::current_branch()?;
    let base = git::base_branch(&cfg.git.base_branch);

    let log = Command::new("git")
        .args(["log", "--oneline", &format!("{base}..HEAD")])
        .output()
        .map_err(|e| VicraftError::git("log", e.to_string(), ""))?;
    let commits = String::from_utf8_lossy(&log.stdout).to_string();

    if commits.trim().is_empty() {
        return Err(VicraftError::validation(format!(
            "No commits found between {base} and HEAD."
        )));
    }

    let diff = git::diff_base_to_head(&base)?;

    let model = cfg.model_for_step("pr");
    println!("{}", "Generating PR description...".bold());
    println!("  Model: {}", model.cyan());
    let prompt = format!(
        r#"Generate a pull request title and description for the following changes.

## Commits
{commits}

## Diff summary
{diff}

Output format (exactly):
TITLE: <conventional commit style title>
---
<markdown PR description with:
- ## Changes section summarizing what was done
- ## Testing section if tests were added
- Reference to spec file if found in commit messages
Keep it concise — 150-300 words total.>
"#
    );

    let result = AiderCommand::ask(&cfg.aider, &prompt)
        .override_model(model)
        .run_capture()?;
    tokens::display_usage(&result.usage);

    let (title, body) = parse_pr_output(&result.stdout, &branch);

    println!();
    println!("{}", format!("PR title: {title}").bold());
    println!("{}", "─".repeat(60));
    println!("{body}");
    println!("{}", "─".repeat(60));
    println!();

    let choice = Select::new("Action:", vec!["Create PR", "Edit description", "Cancel"])
        .prompt()
        .map_err(|e| VicraftError::validation(format!("prompt cancelled: {e}")))?;

    match choice {
        "Create PR" => create_pr(&title, &body, &base)?,
        "Edit description" => {
            let edited_body = edit_in_temp(&body)?;
            create_pr(&title, &edited_body, &base)?;
        }
        _ => {
            println!("{}", "Cancelled.".yellow());
        }
    }

    Ok(())
}

fn parse_pr_output(output: &str, branch: &str) -> (String, String) {
    if let Some(rest) = output.strip_prefix("TITLE:") {
        let mut parts = rest.splitn(2, "---");
        let title = parts.next().unwrap_or("").trim().to_string();
        let body = parts.next().unwrap_or("").trim().to_string();
        if !title.is_empty() {
            return (title, body);
        }
    }
    (
        format!("feat: changes from {branch}"),
        output.trim().to_string(),
    )
}

fn create_pr(title: &str, body: &str, base: &str) -> Result<()> {
    let status = Command::new("gh")
        .args([
            "pr", "create", "--title", title, "--body", body, "--base", base,
        ])
        .status()
        .map_err(|e| {
            VicraftError::external_tool(
                "gh",
                format!("failed to run: {e}"),
                "Check gh installation",
            )
        })?;

    if status.success() {
        println!("{} Pull request created.", "✓".green());
    } else {
        return Err(VicraftError::external_tool(
            "gh",
            "gh pr create failed",
            "Check 'gh auth status' and ensure you are authenticated",
        ));
    }
    Ok(())
}

fn gh_available() -> bool {
    Command::new("gh").arg("--version").output().is_ok()
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
