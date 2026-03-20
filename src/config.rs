use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Supported content languages for generated documents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    En,
    Zh,
}

impl Default for Lang {
    fn default() -> Self {
        Lang::En
    }
}

impl std::fmt::Display for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Lang::En => write!(f, "en"),
            Lang::Zh => write!(f, "zh"),
        }
    }
}

/// specmate project configuration, stored in `.specmate/config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Language for generated document content.
    #[serde(default)]
    pub lang: Lang,
}

impl Default for Config {
    fn default() -> Self {
        Config { lang: Lang::En }
    }
}

impl Config {
    /// Load config from `.specmate/config.yaml` relative to `repo_root`.
    ///
    /// Falls back to default config with a warning if the file is missing
    /// or malformed. Never returns an error — config issues are non-fatal.
    pub fn load(repo_root: &Path) -> Self {
        let path = repo_root.join(".specmate").join("config.yaml");
        match Self::try_load(&path) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Warning: could not read .specmate/config.yaml: {e}. Using defaults.");
                Config::default()
            }
        }
    }

    fn try_load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        let config: Config = serde_yaml::from_str(&content)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(config)
    }

    /// Write config to `.specmate/config.yaml` relative to `repo_root`.
    pub fn save(&self, repo_root: &Path) -> Result<()> {
        let dir = repo_root.join(".specmate");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("config.yaml");
        let content = serde_yaml::to_string(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Return the path to `.specmate/config.yaml` relative to `repo_root`.
    pub fn path(repo_root: &Path) -> PathBuf {
        repo_root.join(".specmate").join("config.yaml")
    }
}
