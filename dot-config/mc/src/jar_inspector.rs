use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use sha1::{Digest, Sha1};
use sha2::Sha512;

use crate::types::ModMetadata;

pub fn compute_sha1(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = hasher.finalize();
    Ok(format!("{hash:x}"))
}

pub fn compute_sha512(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha512::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = hasher.finalize();
    Ok(format!("{hash:x}"))
}

pub fn hash_bytes_sha512(bytes: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    format!("{hash:x}")
}

pub fn read_fabric_mod_json(path: &Path) -> Result<Option<ModMetadata>> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open JAR: {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("failed to read ZIP from JAR: {}", path.display()))?;

    let fabric_mod = archive.by_name("fabric.mod.json");
    let fabric_mod = match fabric_mod {
        Ok(f) => f,
        Err(_) => return Ok(None),
    };

    let metadata: ModMetadata = serde_json::from_reader(fabric_mod)
        .with_context(|| format!("failed to parse fabric.mod.json in {}", path.display()))?;

    Ok(Some(metadata))
}
