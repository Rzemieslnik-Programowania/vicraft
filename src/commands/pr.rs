use anyhow::{bail, Result};
use colored::Colorize;
use inquire::Select;
use std::process::Command;

use crate::aider::AiderCommand;
use crate::config::Config;
use crate::git;

pub async fn run(cfg: &Config) -> Result<()> {
    git::assert_git_repo()?;

    // Verify gh CLI is available
    if !gh_available() {
        bail!(
            "GitHub CLI (gh) not found. Install with:\n  sudo dnf install gh\n  gh auth login"
        );
    }

    let branch = git::current_branch()?;
    let base = git::base_branch(&cfg.git.base_branch);

    // 1. Get all commits on this branch vs base
    let log = Command::new("git")
        .args(["log", "--oneline", &format!("{base}..HEAD")])
        .output()?;
    let commits = String::from_utf8_lossy(&log.stdout).to_string();

    if commits.trim().is_empty() {
        bail!("No commits found between {base} and HEAD.");
    }

    // 2. Get full diff for PR description generation
    let diff = git::diff_base_to_head(&base)?;

    // 3. Generate PR title and description
    println!("{}", "Generating PR description...".bold());
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

    let output = AiderCommand::ask(&cfg.aider, &prompt)
        .run_capture()?;

    let (title, body) = parse_pr_output(&output, &branch);

    // 4. Show proposal
    println!();
    println!("{}", format!("PR title: {title}").bold());
    println!("{}", "─".repeat(60));
    println!("{body}");
    println!("{}", "─".repeat(60));
    println!();

    let choice = Select::new("Action:", vec!["Create PR", "Edit description", "Cancel"])
        .prompt()?;

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
    // Fallback
    (format!("feat: changes from {branch}"), output.trim().to_string())
}

fn create_pr(title: &str, body: &str, base: &str) -> Result<()> {
    let status = Command::new("gh")
        .args(["pr", "create", "--title", title, "--body", body, "--base", base])
        .status()?;

    if status.success() {
        println!("{} Pull request created.", "✓".green());
    } else {
        bail!("gh pr create failed. Check gh auth status.");
    }
    Ok(())
}

fn gh_available() -> bool {
    Command::new("gh").arg("--version").output().is_ok()
}

fn edit_in_temp(initial: &str) -> Result<String> {
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new()?;
    write!(tmp, "{initial}")?;
    tmp.flush()?;
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".into());
    std::process::Command::new(&editor).arg(tmp.path()).status()?;
    Ok(std::fs::read_to_string(tmp.path())?.trim().to_string())
}
