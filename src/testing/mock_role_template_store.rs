use crate::domain::Layer;
use crate::ports::{RoleTemplatePort, ScaffoldFile};

/// Mock role template store for testing.
#[derive(Default)]
#[allow(dead_code)]
pub struct MockRoleTemplatePort {
    scaffold_files: Vec<ScaffoldFile>,
}

#[allow(dead_code)]
impl MockRoleTemplatePort {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_scaffold_files(mut self, files: Vec<ScaffoldFile>) -> Self {
        self.scaffold_files = files;
        self
    }
}

impl RoleTemplatePort for MockRoleTemplatePort {
    fn scaffold_files(&self) -> Vec<ScaffoldFile> {
        self.scaffold_files.clone()
    }

    fn layer_template(&self, _layer: Layer) -> &str {
        ""
    }

    fn generate_role_yaml(&self, role_id: &str, layer: Layer) -> String {
        format!("role: {}\nlayer: {}\n\nprofile:\n  focus: test\n", role_id, layer.dir_name())
    }
}
