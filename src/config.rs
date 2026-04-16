use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub aider: AiderConfig,
    pub models: ModelsConfig,
    pub linear: LinearConfig,
    pub context7: Context7Config,
    pub git: GitConfig,
    pub editor: EditorConfig,
    pub web_search: WebSearchConfig,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ModelsConfig {
    pub spec: String,
    pub plan: String,
    pub implement: String,
    pub review: String,
    pub commit: String,
    pub pr: String,
    pub scan: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AiderConfig {
    pub model: String,
    pub extra_flags: Vec<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct LinearConfig {
    pub api_token: String,
    pub team_id: String,
    /// Defaults to false (opt-in feature).
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

#[derive(Debug, Default, Deserialize, Serialize)]
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

impl Default for ModelsConfig {
    fn default() -> Self {
        Self {
            spec: "ollama/qwen3:32b".into(),
            plan: "ollama/deepseek-r1:32b".into(),
            implement: "ollama/qwen3-coder:30b".into(),
            review: "ollama/glm4:32b".into(),
            commit: "ollama/qwen3:32b".into(),
            pr: "ollama/qwen3:32b".into(),
            scan: "ollama/qwen3-coder:30b".into(),
        }
    }
}

impl Config {
    pub fn model_for_step(&self, step: &str) -> &str {
        match step {
            "spec" => &self.models.spec,
            "plan" => &self.models.plan,
            "implement" => &self.models.implement,
            "review" => &self.models.review,
            "commit" => &self.models.commit,
            "pr" => &self.models.pr,
            "scan" => &self.models.scan,
            other => {
                debug_assert!(false, "model_for_step: unknown step '{other}'");
                &self.aider.model
            }
        }
    }
}

impl Default for AiderConfig {
    fn default() -> Self {
        Self {
            model: "ollama/qwen3-coder:30b".into(),
            extra_flags: vec![
                "--no-auto-commits".into(),
                "--map-tokens".into(),
                "4096".into(),
            ],
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

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "searxng".into(),
            searxng_url: "http://localhost:8080".into(),
        }
    }
}

pub fn config_path() -> Result<PathBuf> {
    let base = dirs_next::config_dir()
        .or_else(|| dirs_next::home_dir().map(|h| h.join(".config")))
        .context("Cannot determine config directory. Set $HOME or $XDG_CONFIG_HOME.")?;
    Ok(base.join("vicraft").join("config.toml"))
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;
    let table: toml::Table = raw
        .parse()
        .with_context(|| format!("Failed to parse config: {}", path.display()))?;
    warn_unknown_model_keys(&table);
    let cfg: Config = table
        .try_into()
        .with_context(|| format!("Failed to deserialize config: {}", path.display()))?;
    Ok(cfg)
}

fn warn_unknown_model_keys(table: &toml::Table) {
    const KNOWN_KEYS: &[&str] = &["spec", "plan", "implement", "review", "commit", "pr", "scan"];
    if let Some(toml::Value::Table(models)) = table.get("models") {
        for key in models.keys() {
            if !KNOWN_KEYS.contains(&key.as_str()) {
                eprintln!(
                    "warning: unknown key `models.{}` in config — valid keys: {}",
                    key,
                    KNOWN_KEYS.join(", ")
                );
            }
        }
    }
}

pub fn save(cfg: &Config) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let raw = toml::to_string_pretty(cfg)?;
    std::fs::write(&path, raw)?;
    Ok(())
}

impl EditorConfig {
    /// Resolves the editor command. Not yet called — reserved for future editor integration.
    #[allow(dead_code)]
    pub fn resolve(&self) -> String {
        if !self.command.is_empty() {
            return self.command.clone();
        }
        std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".into())
    }
}
