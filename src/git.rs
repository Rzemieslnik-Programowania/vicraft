use git2::Repository;
use std::path::Path;
use std::process::Command;

use crate::error::{Result, VicraftError};

pub fn current_branch() -> Result<String> {
    let repo = Repository::open(".")
        .map_err(|_| VicraftError::git("open", "not a git repository", "Run 'git init' first"))?;
    let head = repo.head().map_err(|_| {
        VicraftError::git(
            "read HEAD",
            "could not read HEAD",
            "Check your git repository state",
        )
    })?;
    let name = head
        .shorthand()
        .ok_or_else(|| {
            VicraftError::git(
                "read branch",
                "HEAD is not a named branch (detached HEAD)",
                "Checkout a branch: git checkout <branch>",
            )
        })?
        .to_string();
    Ok(name)
}

pub fn base_branch(configured: &str) -> String {
    configured.to_string()
}

pub fn diff_base_to_head(base: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["diff", &format!("{base}...HEAD")])
        .output()
        .map_err(|e| VicraftError::git("diff", e.to_string(), "Ensure git is installed"))?;
    if !output.status.success() {
        return Err(VicraftError::git(
            "diff",
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
            format!("Verify that branch '{base}' exists"),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn diff_staged(base: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["diff", "--cached", base])
        .output()
        .map_err(|e| {
            VicraftError::git("diff --cached", e.to_string(), "Ensure git is installed")
        })?;
    if !output.status.success() {
        return Err(VicraftError::git(
            "diff --cached",
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
            format!("Verify that branch '{base}' exists"),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn create_branch_if_needed(branch: &str) -> Result<()> {
    let exists = Command::new("git")
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .status()
        .map_err(|e| VicraftError::git("show-ref", e.to_string(), "Ensure git is installed"))?
        .success();

    if !exists {
        let status = Command::new("git")
            .args(["checkout", "-b", branch])
            .status()
            .map_err(|e| {
                VicraftError::git(
                    "checkout -b",
                    e.to_string(),
                    "Check 'git status' for conflicts",
                )
            })?;
        if !status.success() {
            return Err(VicraftError::git(
                "checkout -b",
                format!("failed to create branch: {branch}"),
                "Check 'git status' for conflicts or uncommitted changes",
            ));
        }
    } else {
        let status = Command::new("git")
            .args(["checkout", branch])
            .status()
            .map_err(|e| {
                VicraftError::git(
                    "checkout",
                    e.to_string(),
                    "Check 'git status' for conflicts",
                )
            })?;
        if !status.success() {
            return Err(VicraftError::git(
                "checkout",
                format!("failed to checkout branch: {branch}"),
                "Check 'git status' for conflicts or uncommitted changes",
            ));
        }
    }
    Ok(())
}

pub fn wip_commit(task_id: &str) -> Result<()> {
    let wip_message = format!("wip: {task_id}");

    let head_msg = Command::new("git")
        .args(["log", "-1", "--pretty=%s"])
        .output()
        .map_err(|e| VicraftError::git("log", e.to_string(), ""))?;
    let head_subject = if head_msg.status.success() {
        String::from_utf8_lossy(&head_msg.stdout).trim().to_string()
    } else {
        String::new()
    };

    let status = Command::new("git")
        .args(["add", "-A"])
        .status()
        .map_err(|e| VicraftError::git("add", e.to_string(), ""))?;
    if !status.success() {
        return Err(VicraftError::git(
            "add -A",
            "failed to stage changes",
            "Check 'git status' for issues",
        ));
    }

    if head_subject == wip_message {
        let status = Command::new("git")
            .args(["commit", "--amend", "--no-edit"])
            .status()
            .map_err(|e| {
                VicraftError::git(
                    "commit --amend",
                    e.to_string(),
                    "Check 'git status' for conflicts",
                )
            })?;
        if !status.success() {
            return Err(VicraftError::git(
                "commit --amend",
                "failed to amend WIP commit",
                "Check 'git status' for conflicts",
            ));
        }
        println!("  ✓ Amended WIP commit: {wip_message}");
    } else {
        let status = Command::new("git")
            .args(["commit", "-m", &wip_message])
            .status()
            .map_err(|e| VicraftError::git("commit", e.to_string(), ""))?;
        if !status.success() {
            return Err(VicraftError::git(
                "commit",
                "failed to create WIP commit (nothing to commit?)",
                "Check 'git status' — there may be no changes to commit",
            ));
        }
        println!("  ✓ Created WIP commit: {wip_message}");
    }

    Ok(())
}

pub fn amend_commit(message: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["commit", "--amend", "-m", message])
        .status()
        .map_err(|e| {
            VicraftError::git(
                "commit --amend",
                e.to_string(),
                "Check 'git status' for conflicts",
            )
        })?;
    if !status.success() {
        return Err(VicraftError::git(
            "commit --amend",
            "failed to amend commit",
            "Check 'git status' for conflicts",
        ));
    }
    Ok(())
}

pub fn new_commit(message: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["commit", "-m", message])
        .status()
        .map_err(|e| VicraftError::git("commit", e.to_string(), ""))?;
    if !status.success() {
        return Err(VicraftError::git(
            "commit",
            "failed to create commit (nothing staged?)",
            "Stage files with 'git add' first",
        ));
    }
    Ok(())
}

pub fn task_id_from_branch(branch: &str) -> String {
    branch
        .trim_start_matches("feat/")
        .trim_start_matches("fix/")
        .trim_start_matches("chore/")
        .to_string()
}

#[allow(dead_code)]
pub fn has_changes() -> Result<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map_err(|e| VicraftError::git("status", e.to_string(), "Ensure git is installed"))?;
    Ok(!output.stdout.is_empty())
}

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

pub fn assert_git_repo() -> Result<()> {
    Repository::open(Path::new("."))
        .map_err(|_| VicraftError::git("open", "not a git repository", "Run 'git init' first"))?;
    Ok(())
}
