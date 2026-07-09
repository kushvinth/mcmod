use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    pub version: u32,
    pub minecraft_version: String,
    pub loader: String,
    pub generated_at: String,
    #[serde(default)]
    pub instances: HashMap<String, LockedInstance>,
    #[serde(default)]
    pub managed_paths: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedMod {
    pub project_id: String,
    pub slug: String,
    pub version_id: String,
    pub version_number: String,
    pub filename: String,
    pub sha512: String,
    pub download_url: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedInstance {
    #[serde(default)]
    pub mods: HashMap<String, LockedMod>,
}

impl Lockfile {
    pub fn mod_for(&self, instance: &str, slug: &str) -> Option<&LockedMod> {
        self.instances
            .get(instance)
            .and_then(|inst| inst.mods.get(slug))
    }

    pub fn from_file(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read lockfile: {}", path.display()))?;
        let lockfile: Lockfile = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse lockfile: {}", path.display()))?;
        Ok(Some(lockfile))
    }

    pub fn to_file(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, &content)
            .with_context(|| format!("failed to write lockfile: {}", path.display()))?;
        Ok(())
    }
}
