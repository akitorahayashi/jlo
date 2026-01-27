use crate::domain::{AppError, Layer, RoleId};
use std::path::PathBuf;

/// A discovered role with its layer and ID.
#[derive(Debug, Clone)]
pub struct DiscoveredRole {
    pub layer: Layer,
    pub id: String,
}

/// Port for workspace operations (.jules/ directory management).
pub trait WorkspaceStore {
    /// Check if the workspace exists.
    fn exists(&self) -> bool;

    /// Path to the .jules/ directory.
    fn jules_path(&self) -> PathBuf;

    /// Create the workspace directory structure.
    fn create_structure(&self, scaffold_files: &[super::ScaffoldFile]) -> Result<(), AppError>;

    /// Write the version marker.
    fn write_version(&self, version: &str) -> Result<(), AppError>;

    /// Read the current workspace version.
    fn read_version(&self) -> Result<Option<String>, AppError>;

    /// Check if a role exists in a specific layer.
    fn role_exists_in_layer(&self, layer: Layer, role_id: &RoleId) -> bool;

    /// Discover all existing roles across all layers.
    fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError>;

    /// Find a role by fuzzy matching (prefix match).
    fn find_role_fuzzy(&self, query: &str) -> Result<Option<DiscoveredRole>, AppError>;

    /// Get the directory path for a specific role.
    fn role_path(&self, role: &DiscoveredRole) -> Option<PathBuf>;

    /// Scaffold a new role under a specific layer.
    fn scaffold_role_in_layer(
        &self,
        layer: Layer,
        role_id: &RoleId,
        role_yaml: &str,
        prompt_yaml: Option<&str>,
        has_notes: bool,
    ) -> Result<(), AppError>;
}
