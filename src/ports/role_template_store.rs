use crate::domain::{AppError, BuiltinRoleEntry, Layer};

/// A file embedded in the scaffold bundle.
#[derive(Debug, Clone)]
pub struct ScaffoldFile {
    /// Path relative to the workspace root.
    pub path: String,
    /// File content as UTF-8 text.
    pub content: String,
}

/// Port for accessing role templates and scaffold content.
pub trait RoleTemplateStore {
    /// Get all scaffold files (for workspace initialization and bootstrap).
    fn scaffold_files(&self) -> Vec<ScaffoldFile>;

    /// Get control-plane intent files for `.jlo/` initialization.
    ///
    /// Returns user-owned files (config, role customizations, schedules, setup)
    /// sourced directly from the `.jlo/` scaffold assets.
    fn control_plane_files(&self) -> Vec<ScaffoldFile>;

    /// Get control-plane skeleton files only (config, setup, infrastructure).
    ///
    /// Excludes role definitions and schedules. Used by `update`
    /// to fill missing infrastructure without recreating deleted entities.
    fn control_plane_skeleton_files(&self) -> Vec<ScaffoldFile>;

    /// Get the template for a specific layer.
    #[allow(dead_code)]
    fn layer_template(&self, layer: Layer) -> &str;

    /// Generate role.yml content for a new custom role.
    fn generate_role_yaml(&self, role_id: &str, layer: Layer) -> String;

    /// Load the builtin role catalog.
    fn builtin_role_catalog(&self) -> Result<Vec<BuiltinRoleEntry>, AppError>;

    /// Read builtin role file content by catalog path.
    fn builtin_role_content(&self, path: &str) -> Result<String, AppError>;
}
