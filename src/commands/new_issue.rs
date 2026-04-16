use chrono::Local;
use colored::Colorize;
use std::path::PathBuf;
use std::process::Command;

use crate::error::{Result, VicraftError};
use crate::git::slugify;

pub fn run(name: &str, open: bool) -> Result<()> {
    let date = Local::now().format("%Y-%m-%d");
    let slug = slugify(name);
    if slug.is_empty() {
        return Err(VicraftError::validation(
            "Issue name must contain at least one alphanumeric character.",
        ));
    }
    let filename = format!("{date}-{slug}.md");
    let path = PathBuf::from(".issues").join(&filename);

    if path.exists() {
        return Err(VicraftError::validation(format!(
            "Issue file already exists: {}",
            path.display()
        )));
    }

    let template = std::fs::read_to_string(".aider/templates/ISSUE_TEMPLATE.md").map_err(|_| {
        VicraftError::file_not_found(
            ".aider/templates/ISSUE_TEMPLATE.md",
            "Run 'vicraft init' first",
        )
    })?;

    std::fs::create_dir_all(".issues")
        .map_err(|e| VicraftError::io("new-issue", format!("failed to create .issues/: {e}")))?;
    std::fs::write(&path, &template).map_err(|e| {
        VicraftError::io(
            "new-issue",
            format!("failed to write {}: {e}", path.display()),
        )
    })?;
    println!(
        "{} Created: {}",
        "✓".green(),
        path.display().to_string().yellow()
    );

    if open {
        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".into());
        let status = Command::new(&editor).arg(&path).status().map_err(|e| {
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
    }

    println!();
    println!("👉 Fill in the issue, then run:");
    println!("   {}", format!("vicraft spec {}", path.display()).cyan());

    Ok(())
}
