use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::templates;

pub fn run() -> Result<()> {
    println!("{}", "Initializing vicraft project structure...".bold());

    // Directories
    let dirs = [
        ".aider/skills",
        ".aider/context",
        ".aider/templates",
        ".issues",
        ".specs",
        ".plans",
        ".implementations",
        ".reviews",
    ];
    for dir in &dirs {
        std::fs::create_dir_all(dir)?;
        println!("  {} Created {dir}/", "✓".green());
    }

    // Templates
    let template_files: &[(&str, &str)] = &[
        (".aider/templates/ISSUE_TEMPLATE.md", templates::ISSUE_TEMPLATE),
        (".aider/templates/SPEC_TEMPLATE.md", templates::SPEC_TEMPLATE),
        (".aider/templates/PLAN_TEMPLATE.md", templates::PLAN_TEMPLATE),
        (".aider/templates/REVIEW_TEMPLATE.md", templates::REVIEW_TEMPLATE),
    ];
    for (path, content) in template_files {
        write_if_missing(path, content)?;
        println!("  {} Generated {path}", "✓".green());
    }

    // CONVENTIONS.md skeleton
    write_if_missing(".aider/CONVENTIONS.md", templates::CONVENTIONS_SKELETON)?;
    println!("  {} Generated .aider/CONVENTIONS.md (skeleton — fill it in!)", "✓".green());

    // .aider.conf.yml
    write_if_missing(".aider.conf.yml", templates::AIDER_CONF)?;
    println!("  {} Generated .aider.conf.yml", "✓".green());

    // .gitignore update
    update_gitignore()?;
    println!("  {} Updated .gitignore", "✓".green());

    println!();
    println!("{}", "Next steps:".bold());
    println!("  1. Fill in {}", ".aider/CONVENTIONS.md".yellow());
    println!("  2. Add project-specific skills in {}", ".aider/skills/".yellow());
    println!("  3. Run {} to analyze your codebase", "vicraft scan".cyan());

    Ok(())
}

fn write_if_missing(path: &str, content: &str) -> Result<()> {
    if !Path::new(path).exists() {
        std::fs::write(path, content)?;
    }
    Ok(())
}

fn update_gitignore() -> Result<()> {
    let path = Path::new(".gitignore");
    let entries = "\n# vicraft — auto-generated context (not committed)\n.aider/context/\n";

    if path.exists() {
        let current = std::fs::read_to_string(path)?;
        if !current.contains(".aider/context/") {
            std::fs::write(path, current + entries)?;
        }
    } else {
        std::fs::write(path, entries.trim_start())?;
    }
    Ok(())
}
