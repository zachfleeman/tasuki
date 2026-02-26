use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{Result, TasukiError};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub waybar: WaybarConfig,
    #[serde(default)]
    pub backends: BackendsConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WaybarConfig {
    /// "overdue_today" (default), "all", "today_only"
    #[serde(default = "default_tooltip_scope")]
    pub tooltip_scope: String,
}

impl Default for WaybarConfig {
    fn default() -> Self {
        Self {
            tooltip_scope: default_tooltip_scope(),
        }
    }
}

fn default_tooltip_scope() -> String {
    "overdue_today".into()
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeneralConfig {
    #[serde(default = "default_view")]
    pub default_view: String,
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_view: default_view(),
            theme: default_theme(),
        }
    }
}

fn default_view() -> String {
    "today".into()
}

fn default_theme() -> String {
    "omarchy".into()
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct BackendsConfig {
    #[serde(default)]
    pub obsidian: Option<toml::Table>,
    #[serde(default)]
    pub local: Option<toml::Table>,
}

impl Config {
    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        let config_path = match path {
            Some(p) => p,
            None => Self::default_config_path()?,
        };

        if !config_path.exists() {
            return Ok(Config::default());
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| TasukiError::Config(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }

    pub fn default_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| TasukiError::Config("Could not find config directory".into()))?;
        Ok(config_dir.join("tasuki").join("config.toml"))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            waybar: WaybarConfig::default(),
            backends: BackendsConfig::default(),
        }
    }
}
