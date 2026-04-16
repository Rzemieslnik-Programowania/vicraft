use anyhow::{bail, Context, Result};
use chrono::Local;
use colored::Colorize;

use crate::aider::AiderCommand;
use crate::commands::plan::next_version;
use crate::config::Config;
use crate::git;
use crate::tokens;

pub async fn run(cfg: &Config) -> Result<()> {
    git::assert_git_repo()?;

    // 1. Get branch and task ID
    let branch = git::current_branch()?;
    let task_id = git::task_id_from_branch(&branch);
    let base = git::base_branch(&cfg.git.base_branch);

    println!(
        "{} Branch: {} → diffing against {}",
        "✓".green(),
        branch.yellow(),
        base.yellow()
    );

    // 2. Get diff
    let diff = git::diff_base_to_head(&base)?;
    if diff.trim().is_empty() {
        bail!(
            "No changes found between {base} and HEAD.\n\
             Make sure `vicraft impl` ran and created a WIP commit."
        );
    }

    // 3. Load spec and plan
    let spec_path = format!(".specs/{task_id}_spec.md");
    let spec = std::fs::read_to_string(&spec_path).unwrap_or_else(|_| "(spec not found)".into());

    let plan = find_latest_plan(&task_id).unwrap_or_else(|| "(plan not found)".into());

    let review_template = std::fs::read_to_string(".aider/templates/REVIEW_TEMPLATE.md")
        .context("REVIEW_TEMPLATE.md not found — run `vicraft init` first")?;
    let conventions = std::fs::read_to_string(".aider/CONVENTIONS.md").unwrap_or_default();

    // 4. Build review prompt
    let date = Local::now().format("%Y-%m-%d");
    let prompt = format!(
        r#"Review the diff below and evaluate the implementation against the spec and plan.

## Spec (requirements)
{spec}

## Implementation plan
{plan}

## Diff (only what was implemented in this task)
```diff
{diff}
```

## Project conventions
{conventions}

## Instructions
1. Verify that ALL steps from the plan were completed
2. Check that the code follows CONVENTIONS.md and project skills
3. Verify that acceptance criteria from the spec are satisfied
4. Assess code quality: readability, edge cases, error handling
5. Do NOT suggest changes unrelated to this task
6. Do NOT comment on code that is NOT in the diff
7. Follow the REVIEW_TEMPLATE structure exactly
8. If there are no critical issues, mark Status as "Approved"

## Template
{review_template}

## Template variables
- {{TASK_TITLE}}: {task_id}
- {{BASE_BRANCH}}: {base}
- {{DATE}}: {date}
"#
    );

    // 5. Run review
    let model = cfg.model_for_step("review");
    println!("{}", "Running AI review...".bold());
    println!("  Model: {}", model.cyan());
    let result = AiderCommand::ask(&cfg.aider, &prompt)
        .override_model(model)
        .run_capture()?;
    tokens::display_usage(&result.usage);
    let output = result.stdout;

    // 6. Save review
    std::fs::create_dir_all(".reviews")?;
    let version = next_version(".reviews", &task_id, "review");
    let review_path = format!(".reviews/{task_id}_review{version}.md");
    std::fs::write(&review_path, &output)?;
    println!("{} Review saved: {}", "✓".green(), review_path.yellow());

    // 7. Interpret result
    println!();
    println!("{}", "─".repeat(60));
    println!("{output}");
    println!("{}", "─".repeat(60));

    if is_approved(&output) {
        println!();
        println!("{}", "✅ Review: APPROVED".green().bold());
        println!("👉 {}", "vicraft commit".cyan());
    } else {
        println!();
        println!("{}", "🔄 Review: REQUIRES CHANGES".yellow().bold());
        println!("👉 {}", format!("vicraft plan {review_path}").cyan());
    }

    Ok(())
}

fn find_latest_plan(task_id: &str) -> Option<String> {
    // Try versioned plans in reverse order, then base
    for v in (2..=20).rev() {
        let path = format!(".plans/{task_id}_plan_v{v}.md");
        if let Ok(content) = std::fs::read_to_string(&path) {
            return Some(content);
        }
    }
    std::fs::read_to_string(format!(".plans/{task_id}_plan.md")).ok()
}

fn is_approved(review: &str) -> bool {
    let lower = review.to_lowercase();
    // Look for the Status section approval marker
    lower.contains("**approved**") || lower.contains("[x] **approved**") || {
        // Fallback: no critical issues mentioned
        !lower.contains("🔴") && lower.contains("approved")
    }
}
