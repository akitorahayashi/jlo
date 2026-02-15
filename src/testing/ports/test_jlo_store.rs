//! Test double for `JloStore`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{DiscoveredRole, JloStore};

use super::test_files::TestFiles;

/// In-memory implementation of `JloStore` for unit tests.
///
/// Holds `.jlo/`-scoped state (existence, version, roles) independently
/// from `.jules/`-scoped state in `MockJulesStore`.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MockJloStore {
    files: TestFiles,
    pub exists: Arc<Mutex<bool>>,
    pub version: Arc<Mutex<Option<String>>>,
    pub roles: Arc<Mutex<HashMap<(Layer, RoleId), bool>>>,
}

#[allow(dead_code)]
impl MockJloStore {
    pub fn new(files: TestFiles) -> Self {
        Self {
            files,
            exists: Arc::new(Mutex::new(false)),
            version: Arc::new(Mutex::new(None)),
            roles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_exists(self, exists: bool) -> Self {
        *self.exists.lock().unwrap() = exists;
        self
    }

    pub fn add_role(&self, layer: Layer, role_id: &str) {
        let id = RoleId::new(role_id).expect("Invalid role_id provided in test setup");
        self.roles.lock().unwrap().insert((layer, id), true);
    }
}

impl JloStore for MockJloStore {
    fn jlo_exists(&self) -> bool {
        *self.exists.lock().unwrap()
    }

    fn jlo_path(&self) -> PathBuf {
        PathBuf::from(".jlo")
    }

    fn jlo_write_version(&self, version: &str) -> Result<(), AppError> {
        *self.version.lock().unwrap() = Some(version.to_string());
        Ok(())
    }

    fn jlo_read_version(&self) -> Result<Option<String>, AppError> {
        Ok(self.version.lock().unwrap().clone())
    }

    fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError> {
        let roles: Vec<DiscoveredRole> = self
            .roles
            .lock()
            .unwrap()
            .keys()
            .map(|(layer, id)| DiscoveredRole { layer: *layer, id: id.clone() })
            .collect();
        Ok(roles)
    }

    fn find_role_fuzzy(&self, query: &str) -> Result<Option<DiscoveredRole>, AppError> {
        let roles = self.discover_roles()?;

        if let Some(role) = roles.iter().find(|r| r.id.as_str() == query) {
            return Ok(Some(role.clone()));
        }

        let matches: Vec<_> = roles.iter().filter(|r| r.id.as_str().starts_with(query)).collect();
        match matches.len() {
            1 => Ok(Some(matches[0].clone())),
            _ => Ok(None),
        }
    }

    fn role_path(&self, role: &DiscoveredRole) -> Option<PathBuf> {
        if self.roles.lock().unwrap().contains_key(&(role.layer, role.id.clone())) {
            Some(PathBuf::from(format!(".jlo/roles/{}/{}", role.layer.dir_name(), role.id)))
        } else {
            None
        }
    }

    fn write_role(&self, layer: Layer, role_id: &str, content: &str) -> Result<(), AppError> {
        let path = format!(".jlo/roles/{}/{}/role.yml", layer.dir_name(), role_id);
        self.files.files.lock().unwrap().insert(path, content.to_string());
        Ok(())
    }
}
