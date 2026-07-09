use std::collections::HashMap;

use anyhow::Result;

use crate::types::{DiscoveredMod, MinecraftState, Plan, PlanAction};

fn mod_slug(m: &DiscoveredMod) -> Option<String> {
    m.metadata
        .as_ref()
        .map(|meta| meta.id.clone())
        .or_else(|| crate::modrinth::guess_slug_from_filename(&m.filename))
}

pub fn compute_plan(
    config: &crate::config::Config,
    lockfile: &Option<crate::lockfile::Lockfile>,
    state: &MinecraftState,
) -> Result<Plan> {
    let mut actions = Vec::new();
    let lockfile = lockfile.as_ref();

    for instance_config in &state.instances {
        let inst_cfg = config.instances.get(&instance_config.name);
        let inst_name = &instance_config.name;
        let inst_version = inst_cfg
            .map(|c| c.effective_version(&config.minecraft_version))
            .unwrap_or(&config.minecraft_version);
        let inst_loader = inst_cfg
            .map(|c| c.effective_loader(&config.loader))
            .unwrap_or(&config.loader);

        let mut desired: Vec<String> = Vec::new();
        if let Some(cfg) = inst_cfg {
            for m in &cfg.mods {
                if let Some(slug) = m.slug() {
                    if !desired.contains(&slug.to_string()) {
                        desired.push(slug.to_string());
                    }
                }
            }
        }

        // Separate managed and unmanaged files
        let mut managed: Vec<&DiscoveredMod> = Vec::new();
        let mut unmanaged_by_slug: HashMap<String, &DiscoveredMod> = HashMap::new();

        for m in &instance_config.mods {
            let path_str = m.path.to_string_lossy().to_string();
            let is_managed = lockfile
                .and_then(|lf| lf.managed_paths.get(&path_str))
                .is_some();
            if is_managed {
                managed.push(m);
            } else if let Some(slug) = mod_slug(m) {
                unmanaged_by_slug.insert(slug, m);
                // Also index by filename-derived slug (handles metadata slug mismatch)
                if let Some(fn_slug) = crate::modrinth::guess_slug_from_filename(&m.filename) {
                    unmanaged_by_slug.entry(fn_slug).or_insert(m);
                }
            }
        }

        // Map managed mods by slug (use lockfile slug, not metadata, to handle slug mismatches)
        let managed_by_slug: HashMap<String, &DiscoveredMod> = managed
            .iter()
            .filter_map(|m| {
                let path_str = m.path.to_string_lossy().to_string();
                let slug = lockfile
                    .and_then(|lf| lf.managed_paths.get(&path_str))
                    .cloned()
                    .or_else(|| mod_slug(m));
                slug.map(|s| (s, *m))
            })
            .collect();

        // Check each desired slug
        for slug in &desired {
            if let Some(mod_file) = managed_by_slug.get(slug.as_str()) {
                if let Some(lf) = lockfile {
                    if let Some(entry) = lf.mod_for(inst_name, slug.as_str()) {
                        if mod_file.sha512 != entry.sha512 {
                            actions.push(PlanAction::Update {
                                instance_name: inst_name.clone(),
                                path: mod_file.path.clone(),
                                slug: slug.clone(),
                                _old_sha512: mod_file.sha512.clone(),
                                _new_sha512: entry.sha512.clone(),
                                url: entry.download_url.clone(),
                                filename: entry.filename.clone(),
                            });
                        }
                    } else {
                        add_install_action(&mut actions, inst_name, &instance_config.mods_dir, slug, inst_version, inst_loader, config);
                    }
                }
            } else if let Some(mod_file) = unmanaged_by_slug.get(slug.as_str()) {
                if let Some(lf) = lockfile {
                    if let Some(entry) = lf.mod_for(inst_name, slug.as_str()) {
                        if mod_file.sha512 != entry.sha512 {
                            actions.push(PlanAction::Update {
                                instance_name: inst_name.clone(),
                                path: mod_file.path.clone(),
                                slug: slug.clone(),
                                _old_sha512: mod_file.sha512.clone(),
                                _new_sha512: entry.sha512.clone(),
                                url: entry.download_url.clone(),
                                filename: entry.filename.clone(),
                            });
                        }
                    }
                }
            } else {
                add_install_action(&mut actions, inst_name, &instance_config.mods_dir, slug, inst_version, inst_loader, config);
            }
        }

        // OBSOLETE: managed but not desired
        for (slug, mod_file) in &managed_by_slug {
            if !desired.contains(slug) {
                actions.push(PlanAction::Remove {
                    instance_name: inst_name.clone(),
                    path: mod_file.path.clone(),
                    slug: slug.clone(),
                });
            }
        }
    }

    Ok(Plan { actions })
}

fn add_install_action(
    actions: &mut Vec<PlanAction>,
    inst_name: &str,
    mods_dir: &std::path::Path,
    slug: &str,
    mc_version: &str,
    loader: &str,
    config: &crate::config::Config,
) {
    if let Some(source) = config.mod_sources.get(slug) {
        actions.push(PlanAction::Download {
            instance_name: inst_name.to_string(),
            target_path: mods_dir.join(format!("{slug}.jar")),
            slug: slug.to_string(),
            url: source.url.clone(),
            expected_sha512: source.sha512.clone().unwrap_or_default(),
            filename: format!("{slug}.jar"),
        });
    } else {
        actions.push(PlanAction::Download {
            instance_name: inst_name.to_string(),
            target_path: mods_dir.join(format!("{slug}.jar")),
            slug: slug.to_string(),
            url: format!("__resolve__{slug}__{mc_version}__{loader}"),
            expected_sha512: String::new(),
            filename: format!("{slug}.jar"),
        });
    }
}
