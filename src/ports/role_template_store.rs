use crate::domain::Layer;

/// A file embedded in the scaffold bundle.
#[derive(Debug, Clone)]
pub struct ScaffoldFile {
    /// Path relative to the workspace root.
    pub path: String,
    /// File content as UTF-8 text.
    pub content: String,
}

/// Definition of a built-in role with its content.
#[derive(Debug, Clone)]
pub struct RoleDefinition {
    pub id: &'static str,
    pub layer: Layer,
    pub role_yaml: &'static str,
    pub prompt_yaml: &'static str,
    pub has_notes: bool,
}

/// Port for accessing role templates and scaffold content.
pub trait RoleTemplateStore {
    /// Get all scaffold files (for workspace initialization).
    fn scaffold_files(&self) -> Vec<ScaffoldFile>;

    /// Get all built-in role definitions.
    fn role_definitions(&self) -> &[RoleDefinition];

    /// Look up a built-in role definition by id.
    fn role_definition(&self, role_id: &str) -> Option<&RoleDefinition>;

    /// Get the template for a specific layer.
    fn layer_template(&self, layer: Layer) -> &str;

    /// Generate role.yml content for a new custom role.
    fn generate_role_yaml(&self, role_id: &str, layer: Layer) -> String;

    /// Generate prompt.yml template for a new custom role.
    fn generate_prompt_yaml_template(&self, role_id: &str, layer: Layer) -> String;
}
