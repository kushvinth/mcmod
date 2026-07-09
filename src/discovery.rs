use std::path::PathBuf;

use anyhow::{Context, Result};
use tracing::info;
use walkdir::WalkDir;

use crate::jar_inspector;
use crate::types::{DiscoveredMod, Instance, InstanceSource, MinecraftState};

pub fn find_minecraft_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("unable to determine home directory")?;
    let candidate = home.join("Library/Application Support/Minecraft");
    if candidate.exists() {
        info!(path = %candidate.display(), "found Minecraft directory");
        Ok(candidate)
    } else {
        anyhow::bail!(
            "Minecraft directory not found at {}",
            candidate.display()
        );
    }
}

pub fn scan_minecraft(minecraft_dir: &PathBuf) -> Result<MinecraftState> {
    let mut instances = Vec::new();

    let global_mods = minecraft_dir.join("mods");
    if global_mods.exists() {
        info!("scanning default instance mods: {}", global_mods.display());
        let mods = scan_mods_dir(&global_mods)?;
        instances.push(Instance {
            name: "default".into(),
            source: InstanceSource::Default,
            mods_dir: global_mods,
            mods,
        });
    }

    let instances_dir = minecraft_dir.join("instances");
    if instances_dir.exists() {
        info!("scanning instances directory: {}", instances_dir.display());
        let mut entries: Vec<_> = std::fs::read_dir(&instances_dir)
            .context("failed to read instances directory")?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in &entries {
            let name = entry.file_name().to_string_lossy().to_string();
            let mods_dir = entry.path().join("mods");
            if mods_dir.exists() {
                info!("scanning instance '{name}' mods: {}", mods_dir.display());
                let mods = scan_mods_dir(&mods_dir)?;
                instances.push(Instance {
                    name,
                    source: InstanceSource::Named,
                    mods_dir,
                    mods,
                });
            }
        }
    }

    Ok(MinecraftState {
        minecraft_dir: minecraft_dir.clone(),
        instances,
    })
}

fn scan_mods_dir(mods_dir: &PathBuf) -> Result<Vec<DiscoveredMod>> {
    let mut mods = Vec::new();

    if !mods_dir.exists() {
        return Ok(mods);
    }

    for entry in WalkDir::new(mods_dir).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if !filename.ends_with(".jar") {
            continue;
        }

        let sha1 = crate::jar_inspector::compute_sha1(path)?;
        let sha512 = crate::jar_inspector::compute_sha512(path)?;
        let metadata = jar_inspector::read_fabric_mod_json(path).ok().flatten();

        mods.push(DiscoveredMod {
            path: path.to_path_buf(),
            filename,
            sha1,
            sha512,
            metadata,
        });
    }

    mods.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(mods)
}
