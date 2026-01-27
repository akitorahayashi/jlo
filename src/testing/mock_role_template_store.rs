use crate::domain::Layer;
use crate::ports::{RoleDefinition, RoleTemplateStore, ScaffoldFile};

/// Mock role template store for testing.
#[derive(Default)]
#[allow(dead_code)]
pub struct MockRoleTemplateStore {
    scaffold_files: Vec<ScaffoldFile>,
    role_definitions: Vec<RoleDefinition>,
}

#[allow(dead_code)]
impl MockRoleTemplateStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_scaffold_files(mut self, files: Vec<ScaffoldFile>) -> Self {
        self.scaffold_files = files;
        self
    }

    pub fn with_role_definitions(mut self, definitions: Vec<RoleDefinition>) -> Self {
        self.role_definitions = definitions;
        self
    }
}

impl RoleTemplateStore for MockRoleTemplateStore {
    fn scaffold_files(&self) -> Vec<ScaffoldFile> {
        self.scaffold_files.clone()
    }

    fn role_definitions(&self) -> &[RoleDefinition] {
        // Return empty slice for mock by default
        &[]
    }

    fn role_definition(&self, role_id: &str) -> Option<&RoleDefinition> {
        self.role_definitions.iter().find(|r| r.id == role_id)
    }

    fn layer_template(&self, _layer: Layer) -> &str {
        ""
    }

    fn generate_role_yaml(&self, role_id: &str, layer: Layer) -> String {
        format!("role: {}\nlayer: {}\n", role_id, layer.dir_name())
    }

    fn generate_prompt_yaml_template(&self, role_id: &str, layer: Layer) -> String {
        format!("role: {}\nlayer: {}\nprompt: test\n", role_id, layer.dir_name())
    }
}
