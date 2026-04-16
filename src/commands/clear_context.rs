use anyhow::Result;
use colored::Colorize;

pub fn run() -> Result<()> {
    let targets = [".aider.chat.history.md", ".aider.tags.cache.v3"];
    let mut removed = 0;

    for path in &targets {
        if std::path::Path::new(path).exists() {
            std::fs::remove_file(path)?;
            println!("  {} Removed {path}", "✓".green());
            removed += 1;
        }
    }

    if removed == 0 {
        println!(
            "{}",
            "No Aider context files found — already clean.".yellow()
        );
    } else {
        println!("{}", "Context cleared.".green().bold());
    }

    Ok(())
}
