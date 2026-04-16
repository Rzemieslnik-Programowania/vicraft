use anyhow::{Context, Result};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::AiderConfig;
use crate::tokens::{self, AiderResult};

pub struct AiderCommand<'a> {
    cfg: &'a AiderConfig,
    override_model: Option<String>,
    read_files: Vec<PathBuf>,
    edit_files: Vec<PathBuf>,
    message: String,
}

impl<'a> AiderCommand<'a> {
    /// Ask-only mode: no files are edited. Use for spec/plan/review generation.
    pub fn ask(cfg: &'a AiderConfig, message: impl Into<String>) -> Self {
        Self {
            cfg,
            override_model: None,
            read_files: vec![],
            edit_files: vec![],
            message: message.into(),
        }
    }

    /// Edit mode: files in edit_files may be modified by Aider.
    pub fn edit(cfg: &'a AiderConfig, message: impl Into<String>) -> Self {
        Self {
            cfg,
            override_model: None,
            read_files: vec![],
            edit_files: vec![],
            message: message.into(),
        }
    }

    pub fn override_model(mut self, model: impl Into<String>) -> Self {
        self.override_model = Some(model.into());
        self
    }

    pub fn read(mut self, path: impl AsRef<Path>) -> Self {
        self.read_files.push(path.as_ref().to_owned());
        self
    }

    /// Adds a file to Aider's editable list. Not yet called — reserved for future sub-commands.
    #[allow(dead_code)]
    pub fn with_file(mut self, path: impl AsRef<Path>) -> Self {
        self.edit_files.push(path.as_ref().to_owned());
        self
    }

    /// Run and capture stdout (for ask mode — spec/plan/review).
    pub fn run_capture(&self) -> Result<AiderResult> {
        let output = self
            .build_command()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to run aider — is it installed? (`pip install aider-chat`)")?;

        let stderr_text = String::from_utf8_lossy(&output.stderr).to_string();
        for line in stderr_text.lines() {
            eprintln!("{line}");
        }
        let stderr_lines: Vec<String> = stderr_text.lines().map(String::from).collect();

        if !output.status.success() {
            anyhow::bail!(
                "aider exited with status {}: {}",
                output.status,
                stderr_text
            );
        }

        let usage = tokens::extract_usage_from_stderr(&stderr_lines);
        Ok(AiderResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            usage,
        })
    }

    /// Run interactively with inherited stdio (for impl mode).
    /// The returned `AiderResult::stdout` is always empty because stdout is inherited.
    pub fn run_interactive(&self) -> Result<AiderResult> {
        let mut child = self
            .build_command()
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to run aider — is it installed? (`pip install aider-chat`)")?;

        let stderr = child.stderr.take().expect("stderr was piped");
        let handle = std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            let mut lines = Vec::new();
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        eprintln!("{l}");
                        lines.push(l);
                    }
                    Err(e) => {
                        eprintln!("warning: error reading aider stderr: {e}");
                        break;
                    }
                }
            }
            lines
        });

        let stderr_lines = handle.join().unwrap_or_else(|_| {
            eprintln!("warning: stderr reader thread panicked");
            Vec::new()
        });
        let status = child.wait().context("Failed to wait for aider process")?;

        if !status.success() {
            anyhow::bail!("aider exited with status: {}", status);
        }

        let usage = tokens::extract_usage_from_stderr(&stderr_lines);
        Ok(AiderResult {
            stdout: String::new(),
            usage,
        })
    }

    fn build_command(&self) -> Command {
        let mut cmd = Command::new("aider");
        let model = self.override_model.as_deref().unwrap_or(&self.cfg.model);
        cmd.arg("--model").arg(model);
        cmd.arg("--yes-always");
        cmd.arg("--no-pretty");
        cmd.arg("--no-auto-commits");

        for flag in &self.cfg.extra_flags {
            if flag != "--no-auto-commits" {
                cmd.arg(flag);
            }
        }

        for path in &self.read_files {
            cmd.arg("--read").arg(path);
        }

        cmd.arg("--message").arg(&self.message);

        for path in &self.edit_files {
            cmd.arg(path);
        }

        cmd
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
