use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::AppError;
use crate::ports::ScaffoldFile;

const MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedDefaultsManifest {
    pub schema_version: u32,
    pub files: Vec<ManagedDefaultsEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedDefaultsEntry {
    pub path: String,
    pub sha256: String,
}

impl ManagedDefaultsManifest {
    pub fn from_map(map: BTreeMap<String, String>) -> Self {
        let files =
            map.into_iter().map(|(path, sha256)| ManagedDefaultsEntry { path, sha256 }).collect();
        Self { schema_version: MANIFEST_SCHEMA_VERSION, files }
    }

    pub fn to_map(&self) -> BTreeMap<String, String> {
        self.files.iter().map(|entry| (entry.path.clone(), entry.sha256.clone())).collect()
    }
}

pub fn manifest_path(jules_path: &Path) -> PathBuf {
    jules_path.join(".jlo-managed.yml")
}

pub fn load_manifest(jules_path: &Path) -> Result<Option<ManagedDefaultsManifest>, AppError> {
    let path = manifest_path(jules_path);
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)?;
    let manifest: ManagedDefaultsManifest = serde_yaml::from_str(&content).map_err(|err| {
        AppError::config_error(format!("Failed to parse {}: {}", path.display(), err))
    })?;

    if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
        return Err(AppError::config_error(format!(
            "Unsupported managed defaults schema version: {} (expected {})",
            manifest.schema_version, MANIFEST_SCHEMA_VERSION
        )));
    }

    Ok(Some(manifest))
}

pub fn write_manifest(
    jules_path: &Path,
    manifest: &ManagedDefaultsManifest,
) -> Result<(), AppError> {
    let path = manifest_path(jules_path);
    let content = serde_yaml::to_string(manifest).map_err(|err| {
        AppError::config_error(format!("Failed to serialize {}: {}", path.display(), err))
    })?;
    fs::write(&path, content)?;
    Ok(())
}

pub fn manifest_from_scaffold(scaffold_files: &[ScaffoldFile]) -> ManagedDefaultsManifest {
    let mut map = BTreeMap::new();
    for file in scaffold_files {
        if is_default_role_file(&file.path) {
            map.insert(file.path.clone(), hash_content(&file.content));
        }
    }
    ManagedDefaultsManifest::from_map(map)
}

pub fn is_default_role_file(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();

    // .jules/roles/<layer>/prompt.yml (single-role layers)
    if parts.len() == 4 && parts[0] == ".jules" && parts[1] == "roles" && parts[3] == "prompt.yml" {
        return true;
    }

    // .jules/roles/<layer>/<role>/prompt.yml (multi-role layers)
    if parts.len() == 5 && parts[0] == ".jules" && parts[1] == "roles" && parts[4] == "prompt.yml" {
        return true;
    }

    // .jules/roles/observers/<role>/role.yml
    if parts.len() == 5
        && parts[0] == ".jules"
        && parts[1] == "roles"
        && parts[2] == "observers"
        && parts[4] == "role.yml"
    {
        return true;
    }

    false
}

pub fn hash_file(path: &Path) -> Result<String, AppError> {
    let content = fs::read_to_string(path)?;
    Ok(hash_content(&content))
}

pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{:02x}", byte)).collect()
}
