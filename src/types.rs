use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A discovered mod file on disk
#[derive(Debug, Clone)]
pub struct DiscoveredMod {
    pub path: PathBuf,
    pub filename: String,
    pub sha1: String,
    pub sha512: String,
    pub metadata: Option<ModMetadata>,
}

/// Metadata extracted from fabric.mod.json inside a JAR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModMetadata {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub depends: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceSource {
    Default,
    Named,
}

/// A discovered instance (either root default or instances/<name>)
#[derive(Debug, Clone)]
pub struct Instance {
    pub name: String,
    pub source: InstanceSource,
    pub mods_dir: PathBuf,
    pub mods: Vec<DiscoveredMod>,
}

/// Full filesystem state of the Minecraft directory
#[derive(Debug, Clone)]
pub struct MinecraftState {
    pub minecraft_dir: PathBuf,
    pub instances: Vec<Instance>,
}

/// An action to perform during reconciliation
#[derive(Debug, Clone)]
pub enum PlanAction {
    Download {
        instance_name: String,
        target_path: PathBuf,
        slug: String,
        url: String,
        expected_sha512: String,
        filename: String,
    },
    Remove {
        instance_name: String,
        path: PathBuf,
        slug: String,
    },
    Update {
        instance_name: String,
        path: PathBuf,
        slug: String,
        _old_sha512: String,
        _new_sha512: String,
        url: String,
        filename: String,
    },
}

impl PlanAction {
    pub fn description(&self) -> String {
        match self {
            PlanAction::Download { instance_name, slug, filename, .. } => {
                format!("INSTALL {slug:20} → {instance_name}/{filename}")
            }
            PlanAction::Remove { instance_name, slug, .. } => {
                format!("REMOVE  {slug:20} from {instance_name}")
            }
            PlanAction::Update { instance_name, slug, filename, .. } => {
                format!("UPDATE  {slug:20} → {instance_name}/{filename}")
            }
        }
    }
}

/// The complete reconciliation plan
#[derive(Debug, Clone)]
pub struct Plan {
    pub actions: Vec<PlanAction>,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn print(&self) {
        if self.actions.is_empty() {
            println!("No changes needed — everything is up to date.");
            return;
        }
        println!("Planned actions:");
        for action in &self.actions {
            println!("  {}", action.description());
        }
        println!("\nTotal: {} action(s)", self.actions.len());
    }
}
