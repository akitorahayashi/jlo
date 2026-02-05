use crate::domain::Layer;

/// A file embedded in the scaffold bundle.
#[derive(Debug, Clone)]
pub struct ScaffoldFile {
    /// Path relative to the workspace root.
    pub path: String,
    /// File content as UTF-8 text.
    pub content: String,
}

/// Port for accessing role templates and scaffold content.
pub trait RoleTemplatePort {
    /// Get all scaffold files (for workspace initialization).
    fn scaffold_files(&self) -> Vec<ScaffoldFile>;

    /// Get the template for a specific layer.
    #[allow(dead_code)]
    fn layer_template(&self, layer: Layer) -> &str;

    /// Generate role.yml content for a new custom role.
    fn generate_role_yaml(&self, role_id: &str, layer: Layer) -> String;
}
