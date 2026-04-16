use chrono::Local;
use colored::Colorize;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum AiderErrorKind {
    NotFound,
    ProcessFailed { status: String, stderr: String },
    ModelError { message: String },
}

impl fmt::Display for AiderErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "aider binary not found"),
            Self::ProcessFailed { status, stderr } => {
                write!(f, "aider exited with {status}")?;
                if !stderr.is_empty() {
                    let truncated = truncate_stderr(stderr, 5);
                    write!(f, "\n{truncated}")?;
                }
                Ok(())
            }
            Self::ModelError { message } => write!(f, "aider model error: {message}"),
        }
    }
}

#[derive(Debug)]
pub enum NetworkErrorKind {
    Auth,
    RateLimit,
    NotFound,
    Connectivity,
    Api(String),
}

impl fmt::Display for NetworkErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auth => write!(f, "authentication failed"),
            Self::RateLimit => write!(f, "rate limited"),
            Self::NotFound => write!(f, "resource not found"),
            Self::Connectivity => write!(f, "network connectivity error"),
            Self::Api(msg) => write!(f, "API error: {msg}"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VicraftError {
    #[error("{kind}")]
    Aider { kind: AiderErrorKind },

    #[error("git {operation}: {detail}")]
    Git {
        operation: String,
        detail: String,
        suggestion: String,
    },

    #[error("config error: {detail}")]
    Config {
        path: PathBuf,
        detail: String,
        suggestion: String,
    },

    #[error("{message}")]
    Validation { message: String },

    #[error("file not found: {}", path.display())]
    FileNotFound { path: PathBuf, suggestion: String },

    #[error("{context}: {detail}")]
    Io { context: String, detail: String },

    #[error("network: {kind}: {detail}")]
    Network {
        kind: NetworkErrorKind,
        detail: String,
    },

    #[error("{tool}: {detail}")]
    ExternalTool {
        tool: String,
        detail: String,
        suggestion: String,
    },

    #[error("multiple errors occurred")]
    Multiple { errors: Vec<VicraftError> },

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl VicraftError {
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            Self::Git { suggestion, .. }
            | Self::Config { suggestion, .. }
            | Self::FileNotFound { suggestion, .. }
            | Self::ExternalTool { suggestion, .. } => {
                if suggestion.is_empty() {
                    None
                } else {
                    Some(suggestion)
                }
            }
            Self::Aider { kind } => match kind {
                AiderErrorKind::NotFound => Some("Install aider: pip install aider-chat"),
                AiderErrorKind::ProcessFailed { .. } => {
                    Some("Check aider logs and model configuration")
                }
                AiderErrorKind::ModelError { .. } => {
                    Some("Verify your model is running and accessible")
                }
            },
            Self::Network { kind, .. } => match kind {
                NetworkErrorKind::Auth => Some("Check your API token configuration"),
                NetworkErrorKind::RateLimit => Some("Wait a moment and retry"),
                NetworkErrorKind::NotFound => Some("Verify the resource ID or URL"),
                NetworkErrorKind::Connectivity => Some("Check your network connection"),
                NetworkErrorKind::Api(_) => None,
            },
            Self::Io { .. } => Some("Check file/directory permissions"),
            Self::Internal(_) => Some("Run with --verbose for the full error chain"),
            Self::Validation { .. } | Self::Multiple { .. } => None,
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Config { .. } | Self::Validation { .. } => 2,
            Self::Multiple { errors } => errors.first().map_or(1, |e| e.exit_code()),
            _ => 1,
        }
    }

    pub fn format_error(&self, verbose: bool) {
        match self {
            Self::Multiple { errors } => {
                for (i, err) in errors.iter().enumerate() {
                    if i > 0 {
                        eprintln!();
                    }
                    err.format_single(verbose);
                }
            }
            _ => self.format_single(verbose),
        }
    }

    fn format_single(&self, verbose: bool) {
        eprintln!("{} {}", "Error:".red().bold(), self);

        if let Some(suggestion) = self.suggestion() {
            eprintln!("{} {}", "Suggestion:".yellow(), suggestion);
        }

        if verbose {
            if let Self::Internal(err) = self {
                let chain: Vec<String> = err.chain().skip(1).map(|e| e.to_string()).collect();
                if !chain.is_empty() {
                    eprintln!();
                    eprintln!("{}", "Caused by:".dimmed());
                    for cause in &chain {
                        let safe = scrub_secrets(cause);
                        eprintln!("  {}", safe.dimmed());
                    }
                }
            }
        }
    }

    pub fn log_error_to_file(&self, command_name: &str) {
        let _ = self.try_log(command_name);
    }

    fn try_log(&self, command_name: &str) -> std::io::Result<()> {
        use std::io::Write;

        std::fs::create_dir_all(".vicraft")?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(".vicraft/error.log")?;

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        match self {
            Self::Multiple { errors } => {
                for err in errors {
                    let safe = scrub_secrets(&err.to_string());
                    writeln!(file, "[{timestamp}] [{command_name}] Error: {safe}")?;
                }
            }
            _ => {
                let safe = scrub_secrets(&self.to_string());
                writeln!(file, "[{timestamp}] [{command_name}] Error: {safe}")?;
                if let Self::Internal(err) = self {
                    for cause in err.chain().skip(1) {
                        let safe = scrub_secrets(&cause.to_string());
                        writeln!(file, "  Caused by: {safe}")?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation {
            message: msg.into(),
        }
    }

    pub fn file_not_found(path: impl Into<PathBuf>, suggestion: impl Into<String>) -> Self {
        Self::FileNotFound {
            path: path.into(),
            suggestion: suggestion.into(),
        }
    }

    pub fn git(
        operation: impl Into<String>,
        detail: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self::Git {
            operation: operation.into(),
            detail: detail.into(),
            suggestion: suggestion.into(),
        }
    }

    pub fn config(
        path: impl Into<PathBuf>,
        detail: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self::Config {
            path: path.into(),
            detail: detail.into(),
            suggestion: suggestion.into(),
        }
    }

    pub fn io(context: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Io {
            context: context.into(),
            detail: detail.into(),
        }
    }

    pub fn external_tool(
        tool: impl Into<String>,
        detail: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self::ExternalTool {
            tool: tool.into(),
            detail: detail.into(),
            suggestion: suggestion.into(),
        }
    }

    pub fn aider_not_found() -> Self {
        Self::Aider {
            kind: AiderErrorKind::NotFound,
        }
    }

    pub fn aider_failed(status: impl Into<String>, stderr: impl Into<String>) -> Self {
        Self::Aider {
            kind: AiderErrorKind::ProcessFailed {
                status: status.into(),
                stderr: stderr.into(),
            },
        }
    }

    pub fn aider_model_error(message: impl Into<String>) -> Self {
        Self::Aider {
            kind: AiderErrorKind::ModelError {
                message: message.into(),
            },
        }
    }

    pub fn network(kind: NetworkErrorKind, detail: impl Into<String>) -> Self {
        Self::Network {
            kind,
            detail: detail.into(),
        }
    }
}

pub(crate) fn scrub_secrets(s: &str) -> String {
    let mut result = s.to_string();
    for pattern in &["sk-", "bearer ", "token="] {
        if let Some(idx) = result.to_lowercase().find(pattern) {
            let start = idx + pattern.len();
            let end = result[start..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .map_or(result.len(), |e| start + e);
            result.replace_range(start..end, "[REDACTED]");
        }
    }
    result
}

pub(crate) fn looks_like_credential(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.contains("bearer ")
        || lower.contains("token=")
        || lower.contains("secret")
        || lower.contains("sk-")
        || lower.contains("api key")
        || lower.contains("api_key")
        || lower.contains("authorization")
}

fn truncate_stderr(stderr: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = stderr.lines().collect();
    if lines.len() <= max_lines {
        return stderr.to_string();
    }
    let kept: Vec<&str> = lines[lines.len() - max_lines..].to_vec();
    format!(
        "  ... ({} lines omitted)\n{}",
        lines.len() - max_lines,
        kept.join("\n")
    )
}

pub type Result<T> = std::result::Result<T, VicraftError>;
