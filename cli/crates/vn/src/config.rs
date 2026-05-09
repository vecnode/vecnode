use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub path: PathBuf,
    pub config: AppConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ollama: OllamaConfig,
    pub sessions: SessionsConfig,
    pub prompts: PromptConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub host: String,
    pub model: String,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionsConfig {
    pub dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    pub system: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let sessions_dir = default_sessions_dir();
        Self {
            ollama: OllamaConfig {
                host: "http://127.0.0.1:11434".to_string(),
                model: "llama3.2".to_string(),
                stream: true,
            },
            sessions: SessionsConfig {
                dir: sessions_dir.to_string_lossy().to_string(),
            },
            prompts: PromptConfig {
                system: Some("You are a local offline systems assistant for vecnode.".to_string()),
            },
        }
    }
}

pub fn load_or_init(override_path: Option<PathBuf>) -> Result<LoadedConfig> {
    let path = override_path.unwrap_or_else(default_config_path);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("failed to create config dir: {}", parent.display()))?;
    }

    let config = if path.exists() {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        toml::from_str::<AppConfig>(&content)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?
    } else {
        let cfg = AppConfig::default();
        let toml_content = toml::to_string_pretty(&cfg).context("failed to serialize default config")?;
        fs::write(&path, toml_content)
            .with_context(|| format!("failed to write default config: {}", path.display()))?;
        cfg
    };

    let sessions_dir = expand_tilde(&config.sessions.dir);
    fs::create_dir_all(&sessions_dir)
        .with_context(|| format!("failed to create sessions dir: {}", sessions_dir.display()))?;

    Ok(LoadedConfig { path, config })
}

pub fn default_config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("vn").join("config.toml")
}

fn default_sessions_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("vn").join("sessions")
}

pub fn expand_tilde(input: &str) -> PathBuf {
    if input == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(input));
    }

    if let Some(rest) = input.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }

    Path::new(input).to_path_buf()
}
