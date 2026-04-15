use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use crate::config::AiderConfig;

pub struct AiderCommand<'a> {
    cfg: &'a AiderConfig,
    read_files: Vec<PathBuf>,
    edit_files: Vec<PathBuf>,
    message: String,
}

impl<'a> AiderCommand<'a> {
    /// Ask-only mode: no files are edited. Use for spec/plan/review generation.
    pub fn ask(cfg: &'a AiderConfig, message: impl Into<String>) -> Self {
        Self {
            cfg,
            read_files: vec![],
            edit_files: vec![],
            message: message.into(),
        }
    }

    /// Edit mode: files in edit_files may be modified by Aider.
    pub fn edit(cfg: &'a AiderConfig, message: impl Into<String>) -> Self {
        Self {
            cfg,
            read_files: vec![],
            edit_files: vec![],
            message: message.into(),
        }
    }

    pub fn read(mut self, path: impl AsRef<Path>) -> Self {
        self.read_files.push(path.as_ref().to_owned());
        self
    }

    pub fn with_file(mut self, path: impl AsRef<Path>) -> Self {
        self.edit_files.push(path.as_ref().to_owned());
        self
    }

    /// Run and capture stdout (for ask mode — spec/plan/review).
    pub fn run_capture(&self) -> Result<String> {
        let output = self.build_command()
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .context("Failed to run aider — is it installed? (`pip install aider-chat`)")?;
        self.check_status(&output)?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Run interactively with inherited stdio (for impl mode).
    pub fn run_interactive(&self) -> Result<()> {
        let status = self.build_command()
            .status()
            .context("Failed to run aider — is it installed? (`pip install aider-chat`)")?;
        if !status.success() {
            anyhow::bail!("aider exited with status: {}", status);
        }
        Ok(())
    }

    fn build_command(&self) -> Command {
        let mut cmd = Command::new("aider");
        cmd.arg("--model").arg(&self.cfg.model);
        cmd.arg("--yes-always");
        cmd.arg("--no-pretty");
        cmd.arg("--no-auto-commits");

        for flag in &self.cfg.extra_flags {
            // extra_flags may already include --no-auto-commits; skip duplicates
            if flag != "--no-auto-commits" {
                cmd.arg(flag);
            }
        }

        for path in &self.read_files {
            cmd.arg("--read").arg(path);
        }

        // Always pass the message last
        cmd.arg("--message").arg(&self.message);

        // Editable files come after flags
        for path in &self.edit_files {
            cmd.arg(path);
        }

        cmd
    }

    fn check_status(&self, output: &Output) -> Result<()> {
        if !output.status.success() {
            anyhow::bail!(
                "aider exited with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }
}

/// Returns the default read files that every Aider session loads.
pub fn default_read_files() -> Vec<PathBuf> {
    let candidates = [
        ".aider/CONVENTIONS.md",
        ".aider/context/CODEBASE.md",
        ".aider/context/PATTERNS.md",
    ];
    candidates
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect()
}

/// Returns skill files relevant to the given content (naive keyword match).
pub fn relevant_skills(content: &str) -> Vec<PathBuf> {
    let skills_dir = std::path::Path::new(".aider/skills");
    if !skills_dir.exists() {
        return vec![];
    }

    let keywords: &[(&str, &str)] = &[
        ("database", "SKILL.database.md"),
        ("migration", "SKILL.database.md"),
        ("schema", "SKILL.database.md"),
        ("api", "SKILL.api.md"),
        ("endpoint", "SKILL.api.md"),
        ("rest", "SKILL.api.md"),
        ("test", "SKILL.testing.md"),
        ("spec", "SKILL.testing.md"),
        ("deploy", "SKILL.deployment.md"),
        ("ci", "SKILL.deployment.md"),
        ("ui", "SKILL.ui.md"),
        ("frontend", "SKILL.ui.md"),
        ("component", "SKILL.ui.md"),
    ];

    let content_lower = content.to_lowercase();
    let mut files = vec![skills_dir.join("SKILL.architecture.md")];

    for (keyword, skill_file) in keywords {
        if content_lower.contains(keyword) {
            let path = skills_dir.join(skill_file);
            if path.exists() && !files.contains(&path) {
                files.push(path);
            }
        }
    }

    files.into_iter().filter(|p| p.exists()).collect()
}
