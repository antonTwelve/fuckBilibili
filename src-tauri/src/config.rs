use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub cache_expiration_days: u64,
    pub proxy_url: Option<String>,
    #[serde(default)]
    pub proxy_enabled: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    "light".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            cache_expiration_days: 7,
            proxy_url: None,
            proxy_enabled: false,
            theme: "light".to_string(),
        }
    }
}

pub struct ConfigManager {
    file_path: String,
    config: Mutex<AppConfig>,
}

impl ConfigManager {
    pub fn new(file_path: &str) -> Self {
        let config = if Path::new(file_path).exists() {
            let content = fs::read_to_string(file_path).unwrap_or_else(|_| "{}".to_string());
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            AppConfig::default()
        };

        Self {
            file_path: file_path.to_string(),
            config: Mutex::new(config),
        }
    }

    pub fn get_config(&self) -> AppConfig {
        self.config.lock().unwrap().clone()
    }

    pub fn set_config(&self, new_config: AppConfig) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&new_config).map_err(|e| e.to_string())?;
        fs::write(&self.file_path, json).map_err(|e| e.to_string())?;
        *self.config.lock().unwrap() = new_config;
        Ok(())
    }
}
