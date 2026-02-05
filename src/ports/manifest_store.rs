use std::path::Path;

use crate::domain::manifest::ScaffoldManifest;
use crate::domain::AppError;
use crate::ports::{ScaffoldFile, WorkspaceStore};

pub trait ManifestStore {
    fn load_manifest(
        &self,
        workspace: &impl WorkspaceStore,
        jules_path: &Path,
    ) -> Result<Option<ScaffoldManifest>, AppError>;
    fn write_manifest(
        &self,
        workspace: &impl WorkspaceStore,
        jules_path: &Path,
        manifest: &ScaffoldManifest,
    ) -> Result<(), AppError>;
    fn is_default_role_file(&self, path: &str) -> bool;
    fn hash_content(&self, content: &str) -> String;
    fn manifest_from_scaffold(&self, scaffold_files: &[ScaffoldFile]) -> ScaffoldManifest;
}
