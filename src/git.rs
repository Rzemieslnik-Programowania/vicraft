use anyhow::{Context, Result};
use git2::Repository;
use std::path::Path;
use std::process::Command;

/// Returns the name of the current git branch.
pub fn current_branch() -> Result<String> {
    let repo = Repository::open(".").context("Not a git repository. Run `git init` first.")?;
    let head = repo.head().context("Could not read HEAD")?;
    let name = head
        .shorthand()
        .context("HEAD is not a named branch")?
        .to_string();
    Ok(name)
}

/// Returns the configured base branch.
pub fn base_branch(configured: &str) -> String {
    configured.to_string()
}

/// Returns the unified diff between base branch and HEAD.
pub fn diff_base_to_head(base: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["diff", &format!("{base}...HEAD")])
        .output()
        .context("Failed to run git diff")?;
    if !output.status.success() {
        anyhow::bail!(
            "git diff failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Returns the diff of staged files only.
pub fn diff_staged(base: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["diff", "--cached", base])
        .output()
        .context("Failed to run git diff --cached")?;
    if !output.status.success() {
        anyhow::bail!(
            "git diff --cached failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Creates a branch if it doesn't already exist and checks it out.
pub fn create_branch_if_needed(branch: &str) -> Result<()> {
    // Check if branch exists
    let exists = Command::new("git")
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .status()?
        .success();

    if !exists {
        let status = Command::new("git")
            .args(["checkout", "-b", branch])
            .status()
            .context("Failed to create branch")?;
        if !status.success() {
            anyhow::bail!("Failed to create branch: {branch}");
        }
    } else {
        let status = Command::new("git")
            .args(["checkout", branch])
            .status()
            .context("Failed to checkout branch")?;
        if !status.success() {
            anyhow::bail!("Failed to checkout branch: {branch}");
        }
    }
    Ok(())
}

/// Stages all changes and creates a WIP commit.
/// If a WIP commit for this task already exists as HEAD, amends it instead.
pub fn wip_commit(task_id: &str) -> Result<()> {
    let wip_message = format!("wip: {task_id}");

    // Check if HEAD is already a WIP commit for this task
    let head_msg = Command::new("git")
        .args(["log", "-1", "--pretty=%s"])
        .output()
        .context("Failed to run git log")?;
    let head_subject = if head_msg.status.success() {
        String::from_utf8_lossy(&head_msg.stdout).trim().to_string()
    } else {
        String::new()
    };

    // Stage everything
    let status = Command::new("git")
        .args(["add", "-A"])
        .status()
        .context("git add -A failed")?;
    if !status.success() {
        anyhow::bail!("git add -A failed");
    }

    if head_subject == wip_message {
        // Amend existing WIP commit
        let status = Command::new("git")
            .args(["commit", "--amend", "--no-edit"])
            .status()
            .context("git commit --amend failed")?;
        if !status.success() {
            anyhow::bail!("git commit --amend failed");
        }
        println!("  ✓ Amended WIP commit: {wip_message}");
    } else {
        // Create new WIP commit
        let status = Command::new("git")
            .args(["commit", "-m", &wip_message])
            .status()
            .context("git commit failed")?;
        if !status.success() {
            anyhow::bail!("git commit failed (nothing to commit?)");
        }
        println!("  ✓ Created WIP commit: {wip_message}");
    }

    Ok(())
}

/// Amends HEAD commit with a new message (used by vicraft commit).
pub fn amend_commit(message: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["commit", "--amend", "-m", message])
        .status()
        .context("git commit --amend failed")?;
    if !status.success() {
        anyhow::bail!("git commit --amend failed");
    }
    Ok(())
}

/// Creates a new commit with the given message (used in --staged mode without WIP).
pub fn new_commit(message: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["commit", "-m", message])
        .status()
        .context("git commit failed")?;
    if !status.success() {
        anyhow::bail!("git commit failed (nothing staged?)");
    }
    Ok(())
}

/// Extracts a task ID from a branch name.
/// feat/2026-04-15-user-auth → 2026-04-15-user-auth
pub fn task_id_from_branch(branch: &str) -> String {
    branch
        .trim_start_matches("feat/")
        .trim_start_matches("fix/")
        .trim_start_matches("chore/")
        .to_string()
}

/// Returns true if the working tree or index has any changes.
#[allow(dead_code)]
pub fn has_changes() -> Result<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()?;
    Ok(!output.stdout.is_empty())
}

/// Slugify a string for use as a branch/file name segment.
pub fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Check if we are inside a git repository.
pub fn assert_git_repo() -> Result<()> {
    Repository::open(Path::new(".")).context("Not a git repository. Run `git init` first.")?;
    Ok(())
}
