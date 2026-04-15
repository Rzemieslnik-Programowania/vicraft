use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub aider: AiderConfig,
    pub linear: LinearConfig,
    pub context7: Context7Config,
    pub git: GitConfig,
    pub editor: EditorConfig,
    pub web_search: WebSearchConfig,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AiderConfig {
    pub model: String,
    pub extra_flags: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct LinearConfig {
    pub api_token: String,
    pub team_id: String,
    pub auto_update_status: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Context7Config {
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct GitConfig {
    pub base_branch: String,
    pub branch_prefix: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct EditorConfig {
    /// Override $EDITOR/$VISUAL. Leave empty to auto-detect.
    pub command: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct WebSearchConfig {
    pub enabled: bool,
    pub provider: String,
    pub searxng_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            aider: AiderConfig::default(),
            linear: LinearConfig::default(),
            context7: Context7Config::default(),
            git: GitConfig::default(),
            editor: EditorConfig::default(),
            web_search: WebSearchConfig::default(),
        }
    }
}

impl Default for AiderConfig {
    fn default() -> Self {
        Self {
            model: "ollama/qwen3-coder:30b".into(),
            extra_flags: vec!["--no-auto-commits".into(), "--map-tokens".into(), "4096".into()],
        }
    }
}

impl Default for LinearConfig {
    fn default() -> Self {
        Self {
            api_token: String::new(),
            team_id: String::new(),
            auto_update_status: false,
        }
    }
}

impl Default for Context7Config {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            base_branch: "main".into(),
            branch_prefix: "feat/".into(),
        }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self { command: String::new() }
    }
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "searxng".into(),
            searxng_url: "http://localhost:8080".into(),
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("vicraft")
        .join("config.toml")
}

pub fn load() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;
    let cfg = toml::from_str(&raw)
        .with_context(|| format!("Failed to parse config: {}", path.display()))?;
    Ok(cfg)
}

pub fn save(cfg: &Config) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let raw = toml::to_string_pretty(cfg)?;
    std::fs::write(&path, raw)?;
    Ok(())
}

impl EditorConfig {
    pub fn resolve(&self) -> String {
        if !self.command.is_empty() {
            return self.command.clone();
        }
        std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".into())
    }
}
