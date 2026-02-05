use crate::domain::AppError;
use crate::domain::manifest::{ScaffoldManifest, MANIFEST_SCHEMA_VERSION};
use crate::ports::WorkspaceStore;

const MANIFEST_PATH: &str = ".jules/.jlo-managed.yml";

pub fn load_manifest(workspace: &impl WorkspaceStore) -> Result<Option<ScaffoldManifest>, AppError> {
    if !workspace.resolve_path(MANIFEST_PATH).exists() {
        return Ok(None);
    }

    let content = workspace.read_file(MANIFEST_PATH)?;
    let manifest: ScaffoldManifest = serde_yaml::from_str(&content).map_err(|err| {
        AppError::ParseError { what: MANIFEST_PATH.to_string(), details: err.to_string() }
    })?;

    if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
        return Err(AppError::WorkspaceIntegrity(format!(
            "Unsupported scaffold manifest schema version: {} (expected {})",
            manifest.schema_version, MANIFEST_SCHEMA_VERSION
        )));
    }

    Ok(Some(manifest))
}

pub fn write_manifest(workspace: &impl WorkspaceStore, manifest: &ScaffoldManifest) -> Result<(), AppError> {
    let content = serde_yaml::to_string(manifest).map_err(|err| {
        AppError::InternalError(format!("Failed to serialize {}: {}", MANIFEST_PATH, err))
    })?;
    workspace.write_file(MANIFEST_PATH, &content)?;
    Ok(())
}
