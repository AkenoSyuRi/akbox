use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SelectorKind {
    #[default]
    Inquire,
    Skim,
    Fzf,
}

impl FromStr for SelectorKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "inquire" => Ok(Self::Inquire),
            "skim" => Ok(Self::Skim),
            "fzf" => Ok(Self::Fzf),
            other => Err(format!(
                "unknown selector '{other}', expected inquire|skim|fzf"
            )),
        }
    }
}

impl fmt::Display for SelectorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inquire => write!(f, "inquire"),
            Self::Skim => write!(f, "skim"),
            Self::Fzf => write!(f, "fzf"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real_adb: Option<String>,
    /// Remember the selected device within the current terminal/session scope.
    #[serde(default = "default_prefer_last_device")]
    pub prefer_last_device: bool,
    /// How long a remembered selection stays valid. `0` disables expiry.
    #[serde(default = "default_remember_ttl_secs")]
    pub remember_ttl_secs: u64,
    #[serde(default = "default_show_model")]
    pub show_model: bool,
    #[serde(default)]
    pub selector: SelectorKind,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            real_adb: None,
            prefer_last_device: default_prefer_last_device(),
            remember_ttl_secs: default_remember_ttl_secs(),
            show_model: default_show_model(),
            selector: SelectorKind::default(),
        }
    }
}

fn default_prefer_last_device() -> bool {
    true
}

fn default_remember_ttl_secs() -> u64 {
    12 * 60 * 60
}

fn default_show_model() -> bool {
    true
}

pub fn config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        dirs::config_dir().map(|dir| dir.join("adb-wrapper").join("config.toml"))
    }
    #[cfg(target_os = "macos")]
    {
        dirs::data_dir().map(|dir| dir.join("adb-wrapper").join("config.toml"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        dirs::config_dir().map(|dir| dir.join("adb-wrapper").join("config.toml"))
    }
}

pub fn load_config() -> Config {
    match config_path() {
        Some(path) => match load_config_from_path(&path) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("adb-wrapper: warning: {err:#}, using defaults");
                Config::default()
            }
        },
        None => Config::default(),
    }
}

pub fn load_config_from_path(path: &Path) -> Result<Config> {
    if !path.is_file() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("adb-wrapper: failed to read config: {}", path.display()))?;
    let config: Config = toml::from_str(&content)
        .with_context(|| format!("adb-wrapper: invalid config: {}", path.display()))?;
    Ok(config)
}

#[cfg(test)]
pub fn save_config_to_path(config: &Config, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "adb-wrapper: failed to create config directory: {}",
                parent.display()
            )
        })?;
    }
    let content = toml::to_string_pretty(config)
        .context("adb-wrapper: failed to serialize config")?;
    std::fs::write(path, content).with_context(|| {
        format!(
            "adb-wrapper: failed to write config: {}",
            path.display()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn default_config_values() {
        let config = Config::default();
        assert!(config.prefer_last_device);
        assert_eq!(config.remember_ttl_secs, 12 * 60 * 60);
        assert!(config.show_model);
        assert_eq!(config.selector, SelectorKind::Inquire);
        assert!(config.real_adb.is_none());
    }

    #[test]
    fn roundtrip_toml() {
        let dir = std::env::temp_dir().join("adb_wrapper_config_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");

        let config = Config {
            real_adb: Some("C:/adb/adb.exe".into()),
            prefer_last_device: true,
            remember_ttl_secs: 60,
            show_model: false,
            selector: SelectorKind::Skim,
        };
        save_config_to_path(&config, &path).unwrap();
        let loaded = load_config_from_path(&path).unwrap();
        assert_eq!(loaded, config);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn selector_kind_from_str() {
        assert_eq!("fzf".parse::<SelectorKind>().unwrap(), SelectorKind::Fzf);
        assert!("invalid".parse::<SelectorKind>().is_err());
    }
}
