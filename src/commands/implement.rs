use chrono::Local;
use colored::Colorize;

use crate::aider::{self, AiderCommand};
use crate::config::Config;
use crate::error::{Result, VicraftError};
use crate::git;
use crate::templates;
use crate::tokens;

pub async fn run(plan_path: &str, cfg: &Config) -> Result<()> {
    git::assert_git_repo()?;

    let plan = std::fs::read_to_string(plan_path)
        .map_err(|_| VicraftError::file_not_found(plan_path, "Check the plan file path"))?;

    let task_id = task_id_from_plan(plan_path);

    let branch = format!("{}{task_id}", cfg.git.branch_prefix);
    git::create_branch_if_needed(&branch)?;
    println!("{} On branch: {}", "✓".green(), branch.yellow());

    let spec_path = format!(".specs/{task_id}_spec.md");
    let spec = std::fs::read_to_string(&spec_path).unwrap_or_default();

    let prompt = build_impl_prompt(&plan, &spec);

    let model = cfg.model_for_step("implement");
    println!("{}", "Running Aider implementation...".bold());
    println!("  Model: {}", model.cyan());
    println!();

    let mut cmd = AiderCommand::edit(&cfg.aider, &prompt).override_model(model);

    for path in aider::default_read_files() {
        cmd = cmd.read(path);
    }
    for path in aider::relevant_skills(&(plan.clone() + &spec)) {
        cmd = cmd.read(path);
    }

    let aider_result = cmd.run_interactive();

    match aider_result {
        Ok(result) => {
            tokens::display_usage(&result.usage);

            std::fs::create_dir_all(".implementations").map_err(|e| {
                VicraftError::io("impl", format!("failed to create .implementations/: {e}"))
            })?;
            let impl_path = make_impl_path(&task_id);
            write_impl_summary(&impl_path, &task_id, plan_path, &branch)?;
            println!(
                "{} Implementation summary: {}",
                "✓".green(),
                impl_path.yellow()
            );

            println!("{}", "Creating WIP commit...".bold());
            if let Err(commit_err) = git::wip_commit(&task_id) {
                eprintln!(
                    "{} WIP commit failed: {}",
                    "Warning:".yellow().bold(),
                    commit_err
                );
                eprintln!(
                    "{} Aider changes are in your working tree — commit manually.",
                    "Suggestion:".yellow()
                );
                return Ok(());
            }

            println!();
            println!("{}", "Next: clear context, then review:".bold());
            println!("   {}", "vicraft clear-context".cyan());
            println!("   {}", "vicraft review".cyan());

            Ok(())
        }
        Err(aider_err) => {
            let has_changes = git::has_changes().unwrap_or(false);
            if has_changes {
                eprintln!(
                    "{} Aider failed but left changes in your working tree.",
                    "Warning:".yellow().bold(),
                );
                match git::wip_commit(&task_id) {
                    Ok(()) => {
                        eprintln!(
                            "{} Partial changes saved in a WIP commit. Review and fix manually.",
                            "Note:".yellow(),
                        );
                        return Err(aider_err);
                    }
                    Err(commit_err) => {
                        return Err(VicraftError::Multiple {
                            errors: vec![aider_err, commit_err],
                        });
                    }
                }
            }
            Err(aider_err)
        }
    }
}

fn build_impl_prompt(plan: &str, spec: &str) -> String {
    format!(
        r#"Implement the following plan step by step.

## Plan
{plan}

## Spec (for reference — acceptance criteria)
{spec}

## Instructions
- Implement EVERY step listed in the plan in the given order
- Apply all conventions from CONVENTIONS.md
- Write or update tests for each step as specified
- Do NOT make changes outside the scope defined in the spec
- When done, briefly summarize what was implemented
"#
    )
}

fn task_id_from_plan(plan_path: &str) -> String {
    std::path::Path::new(plan_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("task")
        .split("_plan")
        .next()
        .unwrap_or("task")
        .to_string()
}

fn make_impl_path(task_id: &str) -> String {
    use crate::commands::plan::next_version;
    let v = next_version(".implementations", task_id, "impl");
    format!(".implementations/{task_id}_impl{v}.md")
}

fn write_impl_summary(path: &str, task_id: &str, plan_path: &str, branch: &str) -> Result<()> {
    let date = Local::now().format("%Y-%m-%d %H:%M");
    let content = templates::IMPL_SUMMARY_TEMPLATE
        .replace("{TASK_TITLE}", task_id)
        .replace("{PLAN_FILE}", plan_path)
        .replace("{DATE}", &date.to_string())
        .replace("{BRANCH}", branch);
    std::fs::write(path, content)
        .map_err(|e| VicraftError::io("impl", format!("failed to write {path}: {e}")))?;
    Ok(())
}
