use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::domain::{AppError, Layer, PromptAssetLoader, RoleId};
use crate::ports::{DiscoveredRole, ScaffoldFile, WorkspaceStore};

/// Mock workspace store for testing.
#[derive(Default)]
#[allow(dead_code)]
pub struct MockWorkspaceStore {
    pub exists: RefCell<bool>,
    pub roles: RefCell<HashMap<(Layer, RoleId), bool>>,
    pub version: RefCell<Option<String>>,
    pub created_structure: RefCell<bool>,
    pub files: RefCell<HashMap<String, String>>,
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
        let id = RoleId::new(role_id).expect("Invalid role_id provided in test setup");
        self.roles.borrow_mut().insert((layer, id), true);
    }

    pub fn with_file(self, path: &str, content: &str) -> Self {
        self.files.borrow_mut().insert(path.to_string(), content.to_string());
        self
    }
}

impl PromptAssetLoader for MockWorkspaceStore {
    fn read_asset(&self, path: &Path) -> std::io::Result<String> {
        let path_str = path.to_string_lossy().to_string();
        self.files
            .borrow()
            .get(&path_str)
            .cloned()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Mock file not found"))
    }

    fn asset_exists(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_string();
        self.files.borrow().contains_key(&path_str)
    }

    fn ensure_asset_dir(&self, _path: &Path) -> std::io::Result<()> {
        Ok(())
    }

    fn copy_asset(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        let from_str = from.to_string_lossy().to_string();
        let to_str = to.to_string_lossy().to_string();
        let content = self.files.borrow().get(&from_str).cloned().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Source file not found")
        })?;
        self.files.borrow_mut().insert(to_str, content.clone());
        Ok(content.len() as u64)
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
        self.roles.borrow().contains_key(&(layer, role_id.clone()))
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
        if let Some(role) = roles.iter().find(|r| r.id.as_str() == query) {
            return Ok(Some(role.clone()));
        }

        // Prefix match
        let matches: Vec<_> = roles.iter().filter(|r| r.id.as_str().starts_with(query)).collect();
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
    ) -> Result<(), AppError> {
        self.roles.borrow_mut().insert((layer, role_id.clone()), true);
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

    fn read_file(&self, path: &str) -> Result<String, AppError> {
        self.files.borrow().get(path).cloned().ok_or_else(|| {
            AppError::from(std::io::Error::new(std::io::ErrorKind::NotFound, "Mock file not found"))
        })
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), AppError> {
        self.files.borrow_mut().insert(path.to_string(), content.to_string());
        Ok(())
    }

    fn remove_file(&self, path: &str) -> Result<(), AppError> {
        self.files.borrow_mut().remove(path);
        Ok(())
    }

    fn list_dir(&self, path: &str) -> Result<Vec<PathBuf>, AppError> {
        // Find direct children (files and directories)
        let prefix = if path.ends_with('/') { path.to_string() } else { format!("{}/", path) };
        let path_obj = Path::new(path);
        let mut results = std::collections::HashSet::new();

        for key in self.files.borrow().keys() {
            if key.starts_with(&prefix) {
                let suffix = &key[prefix.len()..];
                if let Some(slash_idx) = suffix.find('/') {
                    // It's a subdirectory
                    let dir_name = &suffix[..slash_idx];
                    results.insert(path_obj.join(dir_name));
                } else {
                    // It's a file directly in this directory
                    results.insert(PathBuf::from(key));
                }
            }
        }

        let mut results_vec: Vec<PathBuf> = results.into_iter().collect();
        results_vec.sort();
        Ok(results_vec)
    }

    fn set_executable(&self, _path: &str) -> Result<(), AppError> {
        Ok(())
    }

    fn file_exists(&self, path: &str) -> bool {
        self.files.borrow().contains_key(path)
    }

    fn is_dir(&self, path: &str) -> bool {
        // Check if it is a prefix of any file
        let prefix = if path.ends_with('/') { path.to_string() } else { format!("{}/", path) };
        self.files.borrow().keys().any(|k| k.starts_with(&prefix))
    }

    fn create_dir_all(&self, _path: &str) -> Result<(), AppError> {
        Ok(())
    }

    fn copy_file(&self, src: &str, dst: &str) -> Result<u64, AppError> {
        let content = self.read_file(src)?;
        self.write_file(dst, &content)?;
        Ok(content.len() as u64)
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    fn canonicalize(&self, path: &str) -> Result<PathBuf, AppError> {
        // Mock canonicalization: just return path if it looks valid
        Ok(PathBuf::from(path))
    }
}
