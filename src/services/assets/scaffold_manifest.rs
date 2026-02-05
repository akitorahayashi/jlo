use std::collections::BTreeMap;
use std::path::Path;

use crate::domain::manifest::{self, ScaffoldManifest};
use crate::domain::AppError;
use crate::ports::{ManifestStore, ScaffoldFile, WorkspaceStore};

pub struct ScaffoldManifestAdapter;

impl ManifestStore for ScaffoldManifestAdapter {
    fn load_manifest(
        &self,
        workspace: &impl WorkspaceStore,
        jules_path: &Path,
    ) -> Result<Option<ScaffoldManifest>, AppError> {
        let manifest_path_rel = jules_path.join(".jlo-managed.yml");
        let manifest_path_str = manifest_path_rel.to_string_lossy();

        if !workspace.resolve_path(&manifest_path_str).exists() {
            return Ok(None);
        }

        let content = workspace.read_file(&manifest_path_str)?;
        let manifest: ScaffoldManifest = serde_yaml::from_str(&content).map_err(|err| {
            AppError::ParseError {
                what: manifest_path_str.to_string(),
                details: err.to_string(),
            }
        })?;

        // Manifest version 1
        if manifest.schema_version != 1 {
            return Err(AppError::WorkspaceIntegrity(format!(
                "Unsupported scaffold manifest schema version: {} (expected {})",
                manifest.schema_version, 1
            )));
        }

        Ok(Some(manifest))
    }

    fn write_manifest(
        &self,
        workspace: &impl WorkspaceStore,
        jules_path: &Path,
        manifest: &ScaffoldManifest,
    ) -> Result<(), AppError> {
        let manifest_path_rel = jules_path.join(".jlo-managed.yml");
        let manifest_path_str = manifest_path_rel.to_string_lossy();

        let content = serde_yaml::to_string(manifest).map_err(|err| {
            AppError::InternalError(format!("Failed to serialize {}: {}", manifest_path_str, err))
        })?;
        workspace.write_file(&manifest_path_str, &content)?;
        Ok(())
    }

    fn is_default_role_file(&self, path: &str) -> bool {
        is_default_role_file(path)
    }

    fn hash_content(&self, content: &str) -> String {
        manifest::hash_content(content)
    }

    fn manifest_from_scaffold(&self, scaffold_files: &[ScaffoldFile]) -> ScaffoldManifest {
        manifest_from_scaffold(scaffold_files)
    }
}

pub fn manifest_from_scaffold(scaffold_files: &[ScaffoldFile]) -> ScaffoldManifest {
    let mut map = BTreeMap::new();
    for file in scaffold_files {
        if is_default_role_file(&file.path) {
            map.insert(file.path.clone(), manifest::hash_content(&file.content));
        }
    }
    ScaffoldManifest::from_map(map)
}

fn is_default_role_file(path: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_default_role_file() {
        assert!(is_default_role_file(".jules/roles/observers/roles/taxonomy/role.yml"));
        assert!(is_default_role_file(".jules/roles/deciders/roles/triage_generic/role.yml"));

        assert!(!is_default_role_file(".jules/roles/planners/prompt.yml"));
        assert!(!is_default_role_file("src/main.rs"));
        assert!(!is_default_role_file(".jules/roles/observers/roles/qa/other.yml"));
    }
}
