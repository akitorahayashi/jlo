use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
}
