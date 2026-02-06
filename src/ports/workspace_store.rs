use crate::domain::{AppError, Layer, RoleId};
use std::path::PathBuf;

/// A discovered role with its layer and ID.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DiscoveredRole {
    pub layer: Layer,
    pub id: RoleId,
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
    #[allow(dead_code)]
    fn read_version(&self) -> Result<Option<String>, AppError>;

    /// Check if a role exists in a specific layer.
    fn role_exists_in_layer(&self, layer: Layer, role_id: &RoleId) -> bool;

    /// Discover all existing roles across all layers.
    #[allow(dead_code)]
    fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError>;

    /// Find a role by fuzzy matching (prefix match).
    #[allow(dead_code)]
    fn find_role_fuzzy(&self, query: &str) -> Result<Option<DiscoveredRole>, AppError>;

    /// Get the directory path for a specific role.
    #[allow(dead_code)]
    fn role_path(&self, role: &DiscoveredRole) -> Option<PathBuf>;

    /// Scaffold a new role under a specific layer.
    fn scaffold_role_in_layer(
        &self,
        layer: Layer,
        role_id: &RoleId,
        role_yaml: &str,
    ) -> Result<(), AppError>;

    /// Create a new workstream directory structure.
    fn create_workstream(&self, name: &str) -> Result<(), AppError>;

    /// List existing workstreams.
    fn list_workstreams(&self) -> Result<Vec<String>, AppError>;

    /// Check if a workstream exists.
    fn workstream_exists(&self, name: &str) -> bool;

    // --- Generic File Operations ---

    /// Read a file as a string.
    fn read_file(&self, path: &str) -> Result<String, AppError>;

    /// Write content to a file.
    fn write_file(&self, path: &str, content: &str) -> Result<(), AppError>;

    /// Remove a file.
    fn remove_file(&self, path: &str) -> Result<(), AppError>;

    /// Create directory recursively.
    fn create_dir_all(&self, path: &str) -> Result<(), AppError>;

    /// Copy a file.
    fn copy_file(&self, src: &str, dst: &str) -> Result<u64, AppError>;

    /// Get the absolute path to a file within the workspace/root.
    fn resolve_path(&self, path: &str) -> PathBuf;

    /// Canonicalize a path (resolve symlinks and absolute path).
    fn canonicalize(&self, path: &str) -> Result<PathBuf, AppError>;
}
