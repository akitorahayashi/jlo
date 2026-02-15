use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::domain::{AppError, Layer, PromptAssetLoader, RoleId};
use crate::ports::{DiscoveredRole, ScaffoldFile, WorkspaceStore};

/// Mock workspace store for testing.
#[derive(Clone)]
#[allow(dead_code)]
pub struct MockWorkspaceStore {
    pub exists: Arc<Mutex<bool>>,
    pub roles: Arc<Mutex<HashMap<(Layer, RoleId), bool>>>,
    pub version: Arc<Mutex<Option<String>>>,
    pub created_structure: Arc<Mutex<bool>>,
    pub files: Arc<Mutex<HashMap<String, String>>>,
}

impl Default for MockWorkspaceStore {
    fn default() -> Self {
        Self {
            exists: Arc::new(Mutex::new(false)),
            roles: Arc::new(Mutex::new(HashMap::new())),
            version: Arc::new(Mutex::new(None)),
            created_structure: Arc::new(Mutex::new(false)),
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[allow(dead_code)]
impl MockWorkspaceStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_exists(self, exists: bool) -> Self {
        *self.exists.lock().unwrap() = exists;
        self
    }

    pub fn add_role(&self, layer: Layer, role_id: &str) {
        let id = RoleId::new(role_id).expect("Invalid role_id provided in test setup");
        self.roles.lock().unwrap().insert((layer, id), true);
    }

    pub fn with_file(self, path: &str, content: &str) -> Self {
        self.files.lock().unwrap().insert(path.to_string(), content.to_string());
        self
    }
}

impl PromptAssetLoader for MockWorkspaceStore {
    fn read_asset(&self, path: &Path) -> std::io::Result<String> {
        let path_str = path.to_string_lossy().to_string();
        self.files
            .lock()
            .unwrap()
            .get(&path_str)
            .cloned()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Mock file not found"))
    }

    fn asset_exists(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_string();
        self.files.lock().unwrap().contains_key(&path_str)
    }

    fn ensure_asset_dir(&self, _path: &Path) -> std::io::Result<()> {
        Ok(())
    }

    fn copy_asset(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        let from_str = from.to_string_lossy().to_string();
        let to_str = to.to_string_lossy().to_string();
        let mut files = self.files.lock().unwrap();
        let content = files.get(&from_str).cloned().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Source file not found")
        })?;
        files.insert(to_str, content.clone());
        Ok(content.len() as u64)
    }
}

impl WorkspaceStore for MockWorkspaceStore {
    fn exists(&self) -> bool {
        *self.exists.lock().unwrap()
    }

    fn jlo_exists(&self) -> bool {
        *self.exists.lock().unwrap()
    }

    fn jules_path(&self) -> PathBuf {
        PathBuf::from(".jules")
    }

    fn jlo_path(&self) -> PathBuf {
        PathBuf::from(".jlo")
    }

    fn create_structure(&self, _scaffold_files: &[ScaffoldFile]) -> Result<(), AppError> {
        *self.created_structure.lock().unwrap() = true;
        *self.exists.lock().unwrap() = true;
        Ok(())
    }

    fn write_version(&self, version: &str) -> Result<(), AppError> {
        *self.version.lock().unwrap() = Some(version.to_string());
        Ok(())
    }

    fn read_version(&self) -> Result<Option<String>, AppError> {
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
        if self.roles.lock().unwrap().contains_key(&(role.layer, role.id.clone())) {
            Some(PathBuf::from(format!(".jules/layers/{}/{}", role.layer.dir_name(), role.id)))
        } else {
            None
        }
    }

    fn read_file(&self, path: &str) -> Result<String, AppError> {
        self.files.lock().unwrap().get(path).cloned().ok_or_else(|| {
            AppError::from(std::io::Error::new(std::io::ErrorKind::NotFound, "Mock file not found"))
        })
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), AppError> {
        self.files.lock().unwrap().insert(path.to_string(), content.to_string());
        Ok(())
    }

    fn remove_file(&self, path: &str) -> Result<(), AppError> {
        self.files.lock().unwrap().remove(path);
        Ok(())
    }

    fn remove_dir_all(&self, path: &str) -> Result<(), AppError> {
        let prefix = if path.ends_with('/') { path.to_string() } else { format!("{}/", path) };
        self.files.lock().unwrap().retain(|key, _| !key.starts_with(&prefix));
        Ok(())
    }

    fn list_dir(&self, path: &str) -> Result<Vec<PathBuf>, AppError> {
        // Find direct children (files and directories)
        let prefix = if path.ends_with('/') { path.to_string() } else { format!("{}/", path) };
        let path_obj = Path::new(path);
        let mut results = std::collections::HashSet::new();

        for key in self.files.lock().unwrap().keys() {
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
        self.files.lock().unwrap().contains_key(path)
    }

    fn is_dir(&self, path: &str) -> bool {
        // Check if it is a prefix of any file
        let prefix = if path.ends_with('/') { path.to_string() } else { format!("{}/", path) };
        self.files.lock().unwrap().keys().any(|k| k.starts_with(&prefix))
    }

    fn create_dir_all(&self, _path: &str) -> Result<(), AppError> {
        Ok(())
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    fn canonicalize(&self, path: &str) -> Result<PathBuf, AppError> {
        // Mock canonicalization: just return path if it looks valid
        Ok(PathBuf::from(path))
    }
}
