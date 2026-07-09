use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub minecraft_version: String,
    pub loader: String,
    #[serde(default)]
    pub instances: HashMap<String, InstanceConfig>,
    #[serde(default)]
    pub mod_sources: HashMap<String, ModSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModSource {
    pub url: String,
    #[serde(default)]
    pub sha512: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModRef {
    Simple(String),
    Detailed {
        slug: Option<String>,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        version: Option<String>,
        #[serde(default)]
        url: Option<String>,
        #[serde(default)]
        path: Option<String>,
    },
}

impl ModRef {
    pub fn slug(&self) -> Option<&str> {
        match self {
            ModRef::Simple(s) => Some(s.as_str()),
            ModRef::Detailed { slug, .. } => slug.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    #[serde(default)]
    pub minecraft_version: Option<String>,
    #[serde(default)]
    pub loader: Option<String>,
    #[serde(default)]
    pub mods: Vec<ModRef>,
}

impl InstanceConfig {
    pub fn effective_version<'a>(&'a self, global: &'a str) -> &'a str {
        self.minecraft_version.as_deref().unwrap_or(global)
    }

    pub fn effective_loader<'a>(&'a self, global: &'a str) -> &'a str {
        self.loader.as_deref().unwrap_or(global)
    }
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config: {}", path.display()))?;
        let config: Config = serde_yaml::from_str(&content)
            .with_context(|| format!("failed to parse config: {}", path.display()))?;
        Ok(config)
    }

    pub fn to_file(&self, path: &Path) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, &content)
            .with_context(|| format!("failed to write config: {}", path.display()))?;
        Ok(())
    }
}
