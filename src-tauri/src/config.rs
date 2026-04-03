use crate::models::FilterConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub filter: FilterConfig,
    pub proxy_port: u16,
    pub dns_port: u16,
    pub tun_enabled: bool,
    pub auto_start: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            filter: FilterConfig::default(),
            proxy_port: 7890,
            dns_port: 53,
            tun_enabled: false,
            auto_start: false,
        }
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let config_path = app_data_dir.join("config.json");
        Self { config_path }
    }

    pub fn load(&self) -> AppConfig {
        if !self.config_path.exists() {
            return AppConfig::default();
        }

        match fs::read_to_string(&self.config_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        }
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), ConfigError> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::Io(e.to_string()))?;
        }

        let content =
            serde_json::to_string_pretty(config).map_err(|e| ConfigError::Serialize(e.to_string()))?;

        fs::write(&self.config_path, content).map_err(|e| ConfigError::Io(e.to_string()))?;

        Ok(())
    }

    pub fn update_filter_config(&self, filter: FilterConfig) -> Result<(), ConfigError> {
        let mut config = self.load();
        config.filter = filter;
        self.save(&config)
    }

    pub fn reset(&self) -> Result<(), ConfigError> {
        self.save(&AppConfig::default())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(String),
    Serialize(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(msg) => write!(f, "IO error: {}", msg),
            ConfigError::Serialize(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}
