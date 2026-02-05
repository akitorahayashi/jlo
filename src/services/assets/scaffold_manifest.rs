use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::AppError;
use crate::ports::{ScaffoldFile, WorkspaceStore};

const MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaffoldManifest {
    pub schema_version: u32,
    pub files: Vec<ScaffoldManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaffoldManifestEntry {
    pub path: String,
    pub sha256: String,
}

impl ScaffoldManifest {
    pub fn from_map(map: BTreeMap<String, String>) -> Self {
        let files =
            map.into_iter().map(|(path, sha256)| ScaffoldManifestEntry { path, sha256 }).collect();
        Self { schema_version: MANIFEST_SCHEMA_VERSION, files }
    }

    pub fn to_map(&self) -> BTreeMap<String, String> {
        self.files.iter().map(|entry| (entry.path.clone(), entry.sha256.clone())).collect()
    }
}

pub fn manifest_path_str() -> String {
    format!(".jules/.jlo-managed.yml")
}

pub fn load_manifest(workspace: &impl WorkspaceStore) -> Result<Option<ScaffoldManifest>, AppError> {
    let path = manifest_path_str();
    if !workspace.path_exists(&path) {
        return Ok(None);
    }

    let content = workspace.read_file(&path)?;
    let manifest: ScaffoldManifest = serde_yaml::from_str(&content).map_err(|err| {
        AppError::ParseError { what: path.clone(), details: err.to_string() }
    })?;

    if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
        return Err(AppError::WorkspaceIntegrity(format!(
            "Unsupported scaffold manifest schema version: {} (expected {})",
            manifest.schema_version, MANIFEST_SCHEMA_VERSION
        )));
    }

    Ok(Some(manifest))
}

pub fn write_manifest(
    workspace: &impl WorkspaceStore,
    manifest: &ScaffoldManifest,
) -> Result<(), AppError> {
    let path = manifest_path_str();
    let content = serde_yaml::to_string(manifest)
        .map_err(|err| AppError::InternalError(format!("Failed to serialize {}: {}", path, err)))?;
    workspace.write_file(&path, &content)?;
    Ok(())
}

pub fn manifest_from_scaffold(scaffold_files: &[ScaffoldFile]) -> ScaffoldManifest {
    let mut map = BTreeMap::new();
    for file in scaffold_files {
        if is_default_role_file(&file.path) {
            map.insert(file.path.clone(), hash_content(&file.content));
        }
    }
    ScaffoldManifest::from_map(map)
}

pub fn is_default_role_file(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();

    // .jules/roles/<layer>/roles/<role>/role.yml (multi-role layers: observers, deciders)
    // Example: .jules/roles/observers/roles/taxonomy/role.yml
    if parts.len() == 6
        && parts[0] == ".jules"
        && parts[1] == "roles"
        && parts[3] == "roles"
        && parts[5] == "role.yml"
    {
        return true;
    }

    false
}

#[allow(dead_code)]
pub fn hash_file(workspace: &impl WorkspaceStore, path: &str) -> Result<String, AppError> {
    let content = workspace.read_file(path)?;
    Ok(hash_content(&content))
}

pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{:02x}", byte)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaffold_manifest_conversion() {
        let mut map = BTreeMap::new();
        map.insert("file1.txt".to_string(), "hash1".to_string());
        map.insert("file2.txt".to_string(), "hash2".to_string());

        let manifest = ScaffoldManifest::from_map(map.clone());
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.files.len(), 2);

        let round_trip_map = manifest.to_map();
        assert_eq!(map, round_trip_map);
    }

    #[test]
    fn test_hash_content() {
        let content = "hello world";
        let hash = hash_content(content);
        // echo -n "hello world" | shasum -a 256
        // b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
        assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }

    #[test]
    fn test_is_default_role_file() {
        // New structure: roles are under roles/ container
        assert!(is_default_role_file(".jules/roles/observers/roles/taxonomy/role.yml"));
        assert!(is_default_role_file(".jules/roles/deciders/roles/triage_generic/role.yml"));

        // Not default role files
        assert!(!is_default_role_file(".jules/roles/planners/prompt.yml")); // prompt.yml is managed
        assert!(!is_default_role_file("src/main.rs"));
        assert!(!is_default_role_file(".jules/roles/observers/roles/qa/other.yml"));
    }
}
