use crate::domain::Layer;
use crate::ports::{RoleTemplateStore, ScaffoldFile};

/// Mock role template store for testing.
#[derive(Default)]
#[allow(dead_code)]
pub struct MockRoleTemplateStore {
    scaffold_files: Vec<ScaffoldFile>,
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
}

impl RoleTemplateStore for MockRoleTemplateStore {
    fn scaffold_files(&self) -> Vec<ScaffoldFile> {
        self.scaffold_files.clone()
    }

    fn control_plane_files(&self) -> Vec<ScaffoldFile> {
        // Filter for .jlo/ files and remap user intent files
        self.scaffold_files.iter().filter(|f| f.path.starts_with(".jlo/")).cloned().collect()
    }

    fn control_plane_skeleton_files(&self) -> Vec<ScaffoldFile> {
        self.control_plane_files()
            .into_iter()
            .filter(|f| !(f.path.ends_with("/role.yml") || f.path.ends_with("/scheduled.toml")))
            .collect()
    }

    fn layer_template(&self, _layer: Layer) -> &str {
        ""
    }

    fn generate_role_yaml(&self, role_id: &str, layer: Layer) -> String {
        format!("role: {}\nlayer: {}\n\nprofile:\n  focus: test\n", role_id, layer.dir_name())
    }
}
