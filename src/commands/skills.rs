use chrono::Local;
use colored::Colorize;
use std::process::Command;

use crate::cli::SkillsAction;
use crate::error::{Result, VicraftError};
use crate::templates;

pub fn run(action: SkillsAction) -> Result<()> {
    match action {
        SkillsAction::List => list(),
        SkillsAction::Edit { name } => edit(&name),
        SkillsAction::New { name } => new_skill(&name),
        SkillsAction::Sync => sync(),
    }
}

fn list() -> Result<()> {
    let dir = std::path::Path::new(".aider/skills");
    if !dir.exists() {
        return Err(VicraftError::file_not_found(
            ".aider/skills",
            "Run 'vicraft init' first",
        ));
    }

    let mut found = false;
    for entry in std::fs::read_dir(dir)
        .map_err(|e| VicraftError::io("skills list", format!("failed to read directory: {e}")))?
    {
        let entry = entry
            .map_err(|e| VicraftError::io("skills list", format!("failed to read entry: {e}")))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
            let name = entry.file_name().to_string_lossy().to_string();
            println!("  {}", name.yellow());
            found = true;
        }
    }

    if !found {
        println!("{}", "No skill files found.".yellow());
        println!("Create one with: {}", "vicraft skills new <name>".cyan());
    }
    Ok(())
}

fn edit(name: &str) -> Result<()> {
    let path = skill_path(name);
    if !std::path::Path::new(&path).exists() {
        return Err(VicraftError::file_not_found(
            &path,
            format!("Create it with: vicraft skills new {name}"),
        ));
    }
    open_editor(&path)
}

fn new_skill(name: &str) -> Result<()> {
    std::fs::create_dir_all(".aider/skills").map_err(|e| {
        VicraftError::io(
            "skills new",
            format!("failed to create .aider/skills/: {e}"),
        )
    })?;
    let path = skill_path(name);

    if std::path::Path::new(&path).exists() {
        return Err(VicraftError::validation(format!(
            "Skill file already exists: {path}"
        )));
    }

    let date = Local::now().format("%Y-%m-%d");
    let content = templates::SKILL_TEMPLATE
        .replace("{SKILL_NAME}", &capitalize(name))
        .replace("{slug}", name)
        .replace("{DATE}", &date.to_string());

    std::fs::write(&path, &content)
        .map_err(|e| VicraftError::io("skills new", format!("failed to write {path}: {e}")))?;
    println!("{} Created: {}", "✓".green(), path.yellow());
    open_editor(&path)
}

fn sync() -> Result<()> {
    println!("{}", "Syncing skills via Aider scan...".bold());
    println!(
        "{}",
        "Tip: vicraft scan also refreshes .aider/context/".yellow()
    );
    println!("Run: {}", "vicraft scan".cyan());
    Ok(())
}

fn skill_path(name: &str) -> String {
    let slug = name.to_lowercase().replace(' ', "-");
    format!(".aider/skills/SKILL.{slug}.md")
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn open_editor(path: &str) -> Result<()> {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".into());
    let status = Command::new(&editor).arg(path).status().map_err(|e| {
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
    Ok(())
}
