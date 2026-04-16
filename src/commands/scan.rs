use colored::Colorize;

use crate::aider::AiderCommand;
use crate::config::Config;
use crate::error::{Result, VicraftError};
use crate::tokens;

const SCAN_PROMPT: &str = r#"Analyze this codebase and produce three markdown files.
Respond with three clearly separated sections, each starting with a line:
=== FILE: <filename> ===

Files to produce:
1. CODEBASE.md — directory structure, main modules, entry points, key abstractions
2. DEPENDENCIES.md — all dependencies with versions and their purpose
3. PATTERNS.md — recurring patterns, conventions, naming rules inferred from the code

Be concise. Each file should be under 400 words. Focus on what an AI assistant needs
to understand before modifying this codebase."#;

pub async fn run(cfg: &Config) -> Result<()> {
    let model = cfg.model_for_step("scan");
    println!("{}", "Scanning codebase...".bold());
    println!("  Model: {}", model.cyan());
    std::fs::create_dir_all(".aider/context")
        .map_err(|e| VicraftError::io("scan", format!("failed to create .aider/context/: {e}")))?;

    let result = AiderCommand::ask(&cfg.aider, SCAN_PROMPT)
        .override_model(model)
        .run_capture()?;
    tokens::display_usage(&result.usage);
    let output = result.stdout;

    let files = parse_sections(&output);

    let targets = [
        ("CODEBASE", ".aider/context/CODEBASE.md"),
        ("DEPENDENCIES", ".aider/context/DEPENDENCIES.md"),
        ("PATTERNS", ".aider/context/PATTERNS.md"),
    ];

    for (key, path) in &targets {
        if let Some(content) = files.iter().find(|(k, _)| k == key).map(|(_, v)| v) {
            std::fs::write(path, content)
                .map_err(|e| VicraftError::io("scan", format!("failed to write {path}: {e}")))?;
            println!("  {} Saved {path}", "✓".green());
        } else if *key == "CODEBASE" {
            std::fs::write(path, &output)
                .map_err(|e| VicraftError::io("scan", format!("failed to write {path}: {e}")))?;
            println!("  {} Saved {path} (raw output)", "✓".green());
        }
    }

    println!();
    println!("Run {} again after major refactors.", "vicraft scan".cyan());
    Ok(())
}

fn parse_sections(output: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut current_key: Option<String> = None;
    let mut current_lines: Vec<&str> = Vec::new();

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("=== FILE: ") {
            if let Some(key) = current_key.take() {
                sections.push((key, current_lines.join("\n")));
                current_lines.clear();
            }
            let name = rest
                .trim_end_matches(" ===")
                .trim_end_matches(".md")
                .to_uppercase();
            current_key = Some(name);
        } else if current_key.is_some() {
            current_lines.push(line);
        }
    }
    if let Some(key) = current_key {
        sections.push((key, current_lines.join("\n")));
    }
    sections
}
