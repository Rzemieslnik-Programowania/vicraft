use anyhow::{bail, Context, Result};
use chrono::Local;
use colored::Colorize;
use std::path::PathBuf;
use std::process::Command;

use crate::config::Config;
use crate::git::slugify;

pub fn run(name: &str, open: bool) -> Result<()> {
    // Build path: .issues/<YYYY-MM-DD>-<slug>.md
    let date = Local::now().format("%Y-%m-%d");
    let slug = slugify(name);
    if slug.is_empty() {
        bail!("Issue name must contain at least one alphanumeric character.");
    }
    let filename = format!("{date}-{slug}.md");
    let path = PathBuf::from(".issues").join(&filename);

    if path.exists() {
        bail!("Issue file already exists: {}", path.display());
    }

    // Load template
    let template = std::fs::read_to_string(".aider/templates/ISSUE_TEMPLATE.md")
        .context("ISSUE_TEMPLATE.md not found — run `vicraft init` first")?;

    // Write file
    std::fs::create_dir_all(".issues")?;
    std::fs::write(&path, &template)?;
    println!("{} Created: {}", "✓".green(), path.display().to_string().yellow());

    // Open in editor if requested
    if open {
        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".into());
        Command::new(&editor).arg(&path).status()
            .with_context(|| format!("Failed to open editor: {editor}"))?;
    }

    println!();
    println!("👉 Fill in the issue, then run:");
    println!("   {}", format!("vicraft spec {}", path.display()).cyan());

    Ok(())
}
