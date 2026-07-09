use std::path::Path;

use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::types::Plan;

/// Execute a reconciliation plan: download, update, and remove mods.
pub async fn execute_plan(plan: &Plan, config: &crate::config::Config, _dry_run: bool) -> Result<()> {
    if plan.is_empty() {
        println!("Nothing to do.");
        return Ok(());
    }

    // Resolve slugs that need API lookup
    let mut actions = plan.actions.clone();

    // Pre-resolve all __resolve__ URLs
    for action in &mut actions {
        if let crate::types::PlanAction::Download { url, slug: _, expected_sha512, filename, target_path, .. } = action {
            if url.starts_with("__resolve__") {
                let body = url.strip_prefix("__resolve__").unwrap_or("");
                let parts: Vec<&str> = body.splitn(3, "__").collect();
                if parts.len() >= 3 {
                    let slug = parts[0];
                    let mc_version = parts[1];
                    let loader = parts[2];
                    info!("resolving {slug} for {mc_version}/{loader} from Modrinth...");
                    match resolve_mod(slug, mc_version, loader, config).await {
                        Ok((resolved_url, sha512, resolved_filename)) => {
                            *url = resolved_url;
                            *expected_sha512 = sha512;
                            *filename = resolved_filename.clone();
                            *target_path = target_path.parent().unwrap_or(Path::new("")).join(&resolved_filename);
                        }
                        Err(e) => {
                            warn!("failed to resolve {slug}: {e:?}");
                            println!("  ⚠ Could not resolve '{slug}' from Modrinth. Skipping.");
                            continue;
                        }
                    }
                }
            }
        }
    }

    // Execute
    for action in &actions {
        match action {
            crate::types::PlanAction::Download { instance_name, target_path, slug, url, expected_sha512, filename } => {
                if url.starts_with("__resolve__") {
                    println!("  ⚠ Skipping {slug} — could not resolve.");
                    continue;
                }
                println!("  ⬇ {slug} → {instance_name}/{filename}");
                download_and_install(target_path, url, expected_sha512).await?;
            }
            crate::types::PlanAction::Update { instance_name, path, slug, url, filename, .. } => {
                let parent = path.parent().unwrap();
                let new_path = parent.join(filename);
                println!("  🔄 {slug} in {instance_name} → {filename}");
                download_and_install(&new_path, url, expected_sha512(path, &slug)).await?;
                // Remove old
                if new_path != *path {
                    tokio::fs::remove_file(path).await?;
                }
            }
            crate::types::PlanAction::Remove { instance_name, path, slug } => {
                println!("  🗑 {slug} from {instance_name}");
                tokio::fs::remove_file(path).await
                    .with_context(|| format!("failed to remove {}", path.display()))?;
            }
        }
    }

    Ok(())
}

fn expected_sha512(_path: &Path, _slug: &str) -> &'static str {
    ""
}

async fn download_and_install(target_path: &Path, url: &str, expected_sha512: &str) -> Result<()> {
    // Ensure parent dir exists
    if let Some(parent) = target_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Download to temp file
    let tmp_dir = tempfile::tempdir().context("failed to create temp directory")?;
    let tmp_path = tmp_dir.path().join(
        target_path
            .file_name()
            .unwrap_or_default(),
    );

    info!("downloading {url}");
    let resp = reqwest::get(url)
        .await
        .with_context(|| format!("failed to download {url}"))?;

    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("download failed with status {status} for {url}");
    }

    let bytes = resp.bytes().await?;

    // Verify SHA if expected
    if !expected_sha512.is_empty() {
        let actual = crate::jar_inspector::hash_bytes_sha512(&bytes);
        if actual != expected_sha512 {
            anyhow::bail!(
                "SHA-512 mismatch for {}\n  expected: {expected_sha512}\n  actual:   {actual}",
                target_path.display()
            );
        }
        info!("SHA-512 verified for {}", target_path.display());
    }

    tokio::fs::write(&tmp_path, &bytes)
        .await
        .with_context(|| format!("failed to write temp file: {}", tmp_path.display()))?;

    // Atomic rename
    tokio::fs::rename(&tmp_path, target_path)
        .await
        .with_context(|| {
            format!(
                "failed to rename {} → {}",
                tmp_path.display(),
                target_path.display()
            )
        })?;

    info!("installed {}", target_path.display());
    Ok(())
}

/// Resolve a mod slug to a specific version download URL + SHA + filename.
/// Falls back to config.mod_sources if Modrinth returns an error.
pub async fn resolve_mod(slug: &str, mc_version: &str, loader: &str, config: &crate::config::Config) -> Result<(String, String, String)> {
    let project = match crate::modrinth::get_project(slug).await {
        Ok(p) => p,
        Err(e) => {
            if let Some(source) = config.mod_sources.get(slug) {
                let sha512 = source.sha512.clone().unwrap_or_default();
                let filename = format!("{slug}.jar");
                return Ok((source.url.clone(), sha512, filename));
            }
            return Err(e);
        }
    };
    let versions = crate::modrinth::get_versions(&project.id, mc_version, loader).await?;

    if versions.is_empty() {
        anyhow::bail!(
            "no compatible version of '{slug}' for Minecraft {mc_version} ({loader})"
        );
    }

    // Find the best version that explicitly includes our game version
    let best = versions
        .iter()
        .filter(|v| v.game_versions.iter().any(|gv| gv == mc_version))
        .find(|v| v.version_type == "release")
        .or_else(|| {
            versions
                .iter()
                .find(|v| v.version_type == "release")
        })
        .unwrap_or(&versions[0]);

    let primary_file = best
        .files
        .iter()
        .find(|f| f.primary)
        .unwrap_or(
            best.files.first()
                .context("version has no files")?,
        );

    let sha512 = primary_file
        .hashes
        .sha512
        .clone()
        .ok_or_else(|| anyhow::anyhow!("version has no SHA-512"))?;

    Ok((primary_file.url.clone(), sha512, primary_file.filename.clone()))
}
