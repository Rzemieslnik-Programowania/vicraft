use anyhow::{Context, Result};
use chrono::Local;
use colored::Colorize;

use crate::aider::{self, AiderCommand};
use crate::config::Config;
use crate::git;
use crate::templates;

pub async fn run(plan_path: &str, cfg: &Config) -> Result<()> {
    git::assert_git_repo()?;

    // 1. Load plan and linked spec
    let plan = std::fs::read_to_string(plan_path)
        .with_context(|| format!("Cannot read plan: {plan_path}"))?;

    let task_id = task_id_from_plan(plan_path);

    // 2. Find or create branch
    let branch = format!("{}{task_id}", cfg.git.branch_prefix);
    git::create_branch_if_needed(&branch)?;
    println!("{} On branch: {}", "✓".green(), branch.yellow());

    // 3. Find spec (best-effort)
    let spec_path = format!(".specs/{task_id}_spec.md");
    let spec = std::fs::read_to_string(&spec_path).unwrap_or_default();

    // 4. Build implementation prompt
    let prompt = build_impl_prompt(&plan, &spec, plan_path);

    // 5. Run Aider in interactive/edit mode
    println!("{}", "Running Aider implementation...".bold());
    println!("  Model: {}", cfg.aider.model.cyan());
    println!();

    let mut cmd = AiderCommand::edit(&cfg.aider, &prompt);

    for path in aider::default_read_files() {
        cmd = cmd.read(path);
    }
    for path in aider::relevant_skills(&(plan.clone() + &spec)) {
        cmd = cmd.read(path);
    }

    cmd.run_interactive()?;

    // 6. Generate implementation summary
    std::fs::create_dir_all(".implementations")?;
    let impl_path = make_impl_path(&task_id);
    write_impl_summary(&impl_path, &task_id, plan_path, &branch)?;
    println!("{} Implementation summary: {}", "✓".green(), impl_path.yellow());

    // 7. WIP commit
    println!("{}", "Creating WIP commit...".bold());
    git::wip_commit(&task_id)?;

    println!();
    println!("{}", "Next: clear context, then review:".bold());
    println!("   {}", "vicraft clear-context".cyan());
    println!("   {}", "vicraft review".cyan());

    Ok(())
}

fn build_impl_prompt(plan: &str, spec: &str, plan_path: &str) -> String {
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
    std::fs::write(path, content)?;
    Ok(())
}
