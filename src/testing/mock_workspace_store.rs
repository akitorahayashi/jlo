use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{DiscoveredRole, ScaffoldFile, WorkspaceStore};

/// Mock workspace store for testing.
#[derive(Default)]
#[allow(dead_code)]
pub struct MockWorkspaceStore {
    pub exists: RefCell<bool>,
    pub roles: RefCell<HashMap<(Layer, String), bool>>,
    pub version: RefCell<Option<String>>,
    pub created_structure: RefCell<bool>,
}

#[allow(dead_code)]
impl MockWorkspaceStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_exists(self, exists: bool) -> Self {
        *self.exists.borrow_mut() = exists;
        self
    }

    pub fn add_role(&self, layer: Layer, role_id: &str) {
        self.roles.borrow_mut().insert((layer, role_id.to_string()), true);
    }
}

impl WorkspaceStore for MockWorkspaceStore {
    fn exists(&self) -> bool {
        *self.exists.borrow()
    }

    fn jules_path(&self) -> PathBuf {
        PathBuf::from(".jules")
    }

    fn create_structure(&self, _scaffold_files: &[ScaffoldFile]) -> Result<(), AppError> {
        *self.created_structure.borrow_mut() = true;
        *self.exists.borrow_mut() = true;
        Ok(())
    }

    fn write_version(&self, version: &str) -> Result<(), AppError> {
        *self.version.borrow_mut() = Some(version.to_string());
        Ok(())
    }

    fn read_version(&self) -> Result<Option<String>, AppError> {
        Ok(self.version.borrow().clone())
    }

    fn role_exists_in_layer(&self, layer: Layer, role_id: &RoleId) -> bool {
        self.roles.borrow().contains_key(&(layer, role_id.as_str().to_string()))
    }

    fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError> {
        let roles: Vec<DiscoveredRole> = self
            .roles
            .borrow()
            .keys()
            .map(|(layer, id)| DiscoveredRole { layer: *layer, id: id.clone() })
            .collect();
        Ok(roles)
    }

    fn find_role_fuzzy(&self, query: &str) -> Result<Option<DiscoveredRole>, AppError> {
        let roles = self.discover_roles()?;

        // Exact match
        if let Some(role) = roles.iter().find(|r| r.id == query) {
            return Ok(Some(role.clone()));
        }

        // Prefix match
        let matches: Vec<_> = roles.iter().filter(|r| r.id.starts_with(query)).collect();
        match matches.len() {
            1 => Ok(Some(matches[0].clone())),
            _ => Ok(None),
        }
    }

    fn role_path(&self, role: &DiscoveredRole) -> Option<PathBuf> {
        if self.roles.borrow().contains_key(&(role.layer, role.id.clone())) {
            Some(PathBuf::from(format!(".jules/roles/{}/{}", role.layer.dir_name(), role.id)))
        } else {
            None
        }
    }

    fn scaffold_role_in_layer(
        &self,
        layer: Layer,
        role_id: &RoleId,
        _role_yaml: &str,
        _prompt_yaml: Option<&str>,
        _has_notes: bool,
    ) -> Result<(), AppError> {
        self.roles.borrow_mut().insert((layer, role_id.as_str().to_string()), true);
        Ok(())
    }

    fn create_workstream(&self, _name: &str) -> Result<(), AppError> {
        Ok(())
    }

    fn list_workstreams(&self) -> Result<Vec<String>, AppError> {
        Ok(vec!["generic".to_string()])
    }

    fn workstream_exists(&self, name: &str) -> bool {
        name == "generic"
    }
}
