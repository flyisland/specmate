use anyhow::{Context, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

/// Supported content languages for generated documents.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    En,
    Zh,
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
    /// Missing config returns the default without warning. Read or parse
    /// failures emit a warning to `stderr` and also fall back to defaults.
    pub fn load_with_warnings<W: Write>(repo_root: &Path, stderr: &mut W) -> Self {
        let path = repo_root.join(".specmate").join("config.yaml");
        match Self::try_load(&path) {
            Ok(config) => config,
            Err(error) if error.is_missing() => Config::default(),
            Err(e) => {
                let _ = writeln!(
                    stderr,
                    "[warn] {}\n       could not read config: {e}\n       -> Using default lang en",
                    path.display()
                );
                Config::default()
            }
        }
    }

    fn try_load(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let config: Config = serde_yaml::from_str(&content)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(config)
    }
}

trait ConfigLoadErrorExt {
    fn is_missing(&self) -> bool;
}

impl ConfigLoadErrorExt for anyhow::Error {
    fn is_missing(&self) -> bool {
        self.chain()
            .filter_map(|cause| cause.downcast_ref::<std::io::Error>())
            .any(|error| error.kind() == std::io::ErrorKind::NotFound)
    }
}
