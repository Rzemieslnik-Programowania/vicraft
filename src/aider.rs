use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::AiderConfig;
use crate::error::{looks_like_credential, scrub_secrets, Result, VicraftError};
use crate::tokens::{self, AiderResult};

pub struct AiderCommand<'a> {
    cfg: &'a AiderConfig,
    override_model: Option<String>,
    read_files: Vec<PathBuf>,
    edit_files: Vec<PathBuf>,
    message: String,
}

impl<'a> AiderCommand<'a> {
    pub fn ask(cfg: &'a AiderConfig, message: impl Into<String>) -> Self {
        Self {
            cfg,
            override_model: None,
            read_files: vec![],
            edit_files: vec![],
            message: message.into(),
        }
    }

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

    #[allow(dead_code)]
    pub fn with_file(mut self, path: impl AsRef<Path>) -> Self {
        self.edit_files.push(path.as_ref().to_owned());
        self
    }

    pub fn run_capture(&self) -> Result<AiderResult> {
        let output = self
            .build_command()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(classify_spawn_error)?;

        let stderr_text = String::from_utf8_lossy(&output.stderr).to_string();
        let stderr_lines: Vec<String> = stderr_text.lines().map(String::from).collect();

        if !output.status.success() {
            return Err(classify_process_error(
                &output.status.to_string(),
                &stderr_text,
            ));
        }

        for line in stderr_text.lines() {
            eprintln!("{line}");
        }

        let usage = tokens::extract_usage_from_stderr(&stderr_lines);
        Ok(AiderResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            usage,
        })
    }

    pub fn run_interactive(&self) -> Result<AiderResult> {
        let mut child = self
            .build_command()
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(classify_spawn_error)?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| VicraftError::aider_failed("internal", "stderr pipe not available"))?;

        let handle = std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            let mut lines = Vec::new();
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if !looks_like_credential(&l) {
                            eprintln!("{l}");
                        }
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
        let status = child
            .wait()
            .map_err(|e| VicraftError::aider_failed("wait", e.to_string()))?;

        if !status.success() {
            let stderr_text = stderr_lines.join("\n");
            return Err(classify_process_error(&status.to_string(), &stderr_text));
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

fn classify_spawn_error(e: std::io::Error) -> VicraftError {
    if e.kind() == std::io::ErrorKind::NotFound {
        VicraftError::aider_not_found()
    } else {
        VicraftError::aider_failed("spawn", e.to_string())
    }
}

fn classify_process_error(status: &str, stderr: &str) -> VicraftError {
    let lower = stderr.to_lowercase();

    if lower.contains("api key")
        || lower.contains("api_key")
        || lower.contains("authentication")
        || lower.contains("unauthorized")
    {
        return VicraftError::aider_model_error(
            "API authentication failed — check your API key or model configuration",
        );
    }

    if lower.contains("rate limit") || lower.contains("429") {
        return VicraftError::aider_model_error(
            "rate limited by the model provider — wait and retry",
        );
    }

    if lower.contains("model not found")
        || lower.contains("does not exist")
        || lower.contains("unknown model")
    {
        return VicraftError::aider_model_error(
            "model not available — check your model configuration",
        );
    }

    if looks_like_credential(stderr) {
        return VicraftError::aider_model_error(
            "API credential error — check your API key or model configuration",
        );
    }

    VicraftError::aider_failed(status, scrub_secrets(stderr))
}

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
