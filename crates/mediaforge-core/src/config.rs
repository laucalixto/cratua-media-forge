use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::enums::{Theme, UiMode};
use crate::i18n::Language;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ffmpeg_path: Option<PathBuf>,
    pub language: Language,
    pub theme: Theme,
    pub default_mode: UiMode,
    pub output_dir: Option<PathBuf>,
    pub default_preset: Option<String>,
    pub parallel_jobs: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ffmpeg_path: None,
            language: Language::EnUs,
            theme: Theme::Dark,
            default_mode: UiMode::Simple,
            output_dir: None,
            default_preset: None,
            parallel_jobs: 1,
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        dirs_next().unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("mediaforge").join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| toml::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), crate::error::MediaForgeError> {
        let dir = Self::config_dir().join("mediaforge");
        std::fs::create_dir_all(&dir)?;
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::error::MediaForgeError::Config(e.to_string()))?;
        std::fs::write(Self::config_path(), content)?;
        Ok(())
    }
}

fn dirs_next() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".config"))
            })
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        dirs::config_dir()
    }
}
