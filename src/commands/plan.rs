use chrono::Local;
use colored::Colorize;

use crate::aider::{self, AiderCommand};
use crate::config::Config;
use crate::error::{Result, VicraftError};
use crate::tokens;

pub async fn run(input: &str, cfg: &Config) -> Result<()> {
    let (spec_path, review_content) = resolve_input(input)?;

    let spec = std::fs::read_to_string(&spec_path).map_err(|_| {
        VicraftError::file_not_found(&spec_path, "Check the path or generate a spec first")
    })?;

    let unanswered = unanswered_open_questions(&spec);
    if !unanswered.is_empty() {
        eprintln!(
            "{}",
            "⛔ Spec contains unanswered open questions.".red().bold()
        );
        eprintln!("   Answer them in the spec file before running `vicraft plan`:\n");
        for q in &unanswered {
            eprintln!("   {}", q.yellow());
        }
        eprintln!();
        eprintln!("Edit: {}", spec_path.yellow());
        return Err(VicraftError::validation("Resolve open questions first."));
    }

    let task_id = task_id_from_spec(&spec_path);
    let version = next_version(".plans", &task_id, "plan");
    std::fs::create_dir_all(".plans")
        .map_err(|e| VicraftError::io("plan", format!("failed to create .plans/: {e}")))?;
    let plan_path = format!(".plans/{task_id}_plan{version}.md");

    let plan_template =
        std::fs::read_to_string(".aider/templates/PLAN_TEMPLATE.md").map_err(|_| {
            VicraftError::file_not_found(
                ".aider/templates/PLAN_TEMPLATE.md",
                "Run 'vicraft init' first",
            )
        })?;
    let conventions = std::fs::read_to_string(".aider/CONVENTIONS.md").unwrap_or_default();
    let codebase = std::fs::read_to_string(".aider/context/CODEBASE.md").unwrap_or_default();
    let patterns = std::fs::read_to_string(".aider/context/PATTERNS.md").unwrap_or_default();

    let date = Local::now().format("%Y-%m-%d");

    let review_section = if let Some(review) = &review_content {
        format!("\n## Previous review (address these issues)\n{review}\n")
    } else {
        String::new()
    };

    let prompt = format!(
        r#"Generate a detailed implementation plan based on the spec below.
Follow the PLAN_TEMPLATE structure exactly.

## Spec
{spec}
{review_section}
## Project conventions
{conventions}

## Codebase
{codebase}

## Patterns
{patterns}

## Template
{plan_template}

## Instructions
- List every file that needs to be modified or created, with the exact path
- Each step must be independently reviewable
- Include specific test cases to write for each step
- Do NOT go beyond the scope defined in the spec
- Set {{SPEC_FILE}} to: {spec_path}
- Set {{DATE}} to: {date}
"#
    );

    let model = cfg.model_for_step("plan");
    println!("{}", "Generating plan with Aider...".bold());
    println!("  Model: {}", model.cyan());
    let mut cmd = AiderCommand::ask(&cfg.aider, &prompt).override_model(model);

    for path in aider::default_read_files() {
        cmd = cmd.read(path);
    }
    for path in aider::relevant_skills(&spec) {
        cmd = cmd.read(path);
    }

    let result = cmd.run_capture()?;
    tokens::display_usage(&result.usage);
    let output = result.stdout;

    std::fs::write(&plan_path, &output)
        .map_err(|e| VicraftError::io("plan", format!("failed to write {plan_path}: {e}")))?;
    println!("{} Plan saved: {}", "✓".green(), plan_path.yellow());

    println!();
    println!("{}", "Next: review the plan, then run:".bold());
    println!("   {}", format!("vicraft impl {plan_path}").cyan());

    Ok(())
}

pub fn unanswered_open_questions(spec: &str) -> Vec<String> {
    let mut in_section = false;
    spec.lines()
        .filter(|line| {
            if line.starts_with("## 9. Open questions") {
                in_section = true;
                return false;
            }
            if in_section && line.starts_with("## ") {
                in_section = false;
            }
            in_section && line.trim_start().starts_with("- [ ]")
        })
        .map(|l| l.trim().to_string())
        .collect()
}

fn resolve_input(input: &str) -> Result<(String, Option<String>)> {
    if input.contains("_review") && input.starts_with(".reviews/") {
        let review_content = std::fs::read_to_string(input)
            .map_err(|_| VicraftError::file_not_found(input, "Check the review file path"))?;

        let task_id = std::path::Path::new(input)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .split("_review")
            .next()
            .unwrap_or("")
            .to_string();

        let spec_path = format!(".specs/{task_id}_spec.md");
        if !std::path::Path::new(&spec_path).exists() {
            return Err(VicraftError::file_not_found(
                &spec_path,
                format!("Pass the spec path explicitly: vicraft plan {spec_path}"),
            ));
        }

        println!("{} Iteration mode — using review: {}", "↺".blue(), input);
        println!("{} Spec: {}", "→".blue(), spec_path);
        Ok((spec_path, Some(review_content)))
    } else {
        Ok((input.to_string(), None))
    }
}

fn task_id_from_spec(spec_path: &str) -> String {
    std::path::Path::new(spec_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("task")
        .trim_end_matches("_spec")
        .to_string()
}

pub fn next_version(dir: &str, task_id: &str, suffix: &str) -> String {
    let base = format!("{dir}/{task_id}_{suffix}.md");
    if !std::path::Path::new(&base).exists() {
        return String::new();
    }
    for v in 2..=20 {
        let path = format!("{dir}/{task_id}_{suffix}_v{v}.md");
        if !std::path::Path::new(&path).exists() {
            return format!("_v{v}");
        }
    }
    "_v99".to_string()
}
