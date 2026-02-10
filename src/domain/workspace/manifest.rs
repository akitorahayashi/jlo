//! Scaffold manifest domain entity.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::AppError;

const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// The filename of the managed scaffold manifest.
pub const MANIFEST_FILENAME: &str = ".jlo-managed.yml";

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

    pub fn from_yaml(content: &str) -> Result<Self, AppError> {
        serde_yaml::from_str(content)
            .map_err(|err| AppError::InternalError(format!("Failed to parse manifest: {}", err)))
    }

    pub fn to_yaml(&self) -> Result<String, AppError> {
        serde_yaml::to_string(self).map_err(|err| {
            AppError::InternalError(format!("Failed to serialize manifest: {}", err))
        })
    }
}

pub fn is_default_role_file(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();

    // .jules/roles/<layer>/<role>/role.yml (multi-role layers: observers, innovators)
    // Example: .jules/roles/observers/taxonomy/role.yml
    if parts.len() == 5
        && parts[0] == ".jules"
        && parts[1] == "roles"
        && matches!(parts[2], "observers" | "innovators")
        && parts[4] == "role.yml"
    {
        return true;
    }

    false
}

pub fn is_control_plane_entity_file(path: &str) -> bool {
    let path_obj = Path::new(path);
    let components: Vec<_> =
        path_obj.components().map(|c| c.as_os_str().to_str().unwrap_or("")).collect();

    // .jlo/roles/<layer>/<role>/role.yml
    if components.len() == 5
        && components[0] == ".jlo"
        && components[1] == "roles"
        && components[4] == "role.yml"
    {
        return true;
    }

    // .jlo/scheduled.toml (root schedule)
    if components.len() == 2 && components[0] == ".jlo" && components[1] == "scheduled.toml" {
        return true;
    }

    false
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
        // Multi-role layers: .jules/roles/<layer>/<role>/role.yml
        assert!(is_default_role_file(".jules/roles/observers/taxonomy/role.yml"));
        assert!(is_default_role_file(".jules/roles/innovators/leverage_architect/role.yml"));

        // Deciders is single-role — not a default role file in this sense
        assert!(!is_default_role_file(".jules/roles/deciders/role.yml"));

        // Not default role files
        assert!(!is_default_role_file(".jules/roles/planners/contracts.yml"));
        assert!(!is_default_role_file("src/main.rs"));
        assert!(!is_default_role_file(".jules/roles/observers/qa/other.yml"));
    }
}
