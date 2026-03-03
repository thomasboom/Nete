use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncConfig {
    pub enabled: bool,
    pub endpoint: Option<String>,
}

impl Default for CloudSyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub plugin_dir: PathBuf,
    pub autosave_interval_ms: u64,
    pub cloud_sync: CloudSyncConfig,
}

impl AppConfig {
    pub fn load_or_default() -> AppResult<Self> {
        let dirs = ProjectDirs::from("dev", "nete", "Nete")
            .ok_or_else(|| crate::error::AppError::Invalid("unable to resolve app dirs".into()))?;

        let data_dir = dirs.data_dir().to_path_buf();
        let config_dir = dirs.config_dir().to_path_buf();
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.toml");
        if config_path.exists() {
            let raw = fs::read_to_string(&config_path)?;
            let mut cfg: AppConfig = toml::from_str(&raw)?;
            cfg.ensure_dirs()?;
            return Ok(cfg);
        }

        let cfg = Self::default_with_data_dir(&data_dir);
        cfg.save(&config_path)?;
        Ok(cfg)
    }

    pub fn save(&self, path: &Path) -> AppResult<()> {
        let serialized = toml::to_string_pretty(self)?;
        fs::write(path, serialized)?;
        Ok(())
    }

    fn default_with_data_dir(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            db_path: data_dir.join("nete.sqlite"),
            plugin_dir: data_dir.join("plugins"),
            autosave_interval_ms: 1500,
            cloud_sync: CloudSyncConfig::default(),
        }
    }

    fn ensure_dirs(&mut self) -> AppResult<()> {
        fs::create_dir_all(&self.data_dir)?;
        fs::create_dir_all(&self.plugin_dir)?;
        Ok(())
    }
}

