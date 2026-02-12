use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::domain::workspace::paths::{jules};
use crate::domain::{AppError, JLO_DIR, JULES_DIR, PromptAssetLoader, VERSION_FILE};
use crate::ports::{DiscoveredRole, ScaffoldFile, WorkspaceStore};

/// In-memory workspace store for testing.
#[derive(Debug, Clone)]
pub struct MemoryWorkspaceStore {
    // Using Arc<Mutex> to allow cloning and shared state modification
    files: Arc<Mutex<HashMap<PathBuf, Vec<u8>>>>,
}

impl MemoryWorkspaceStore {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self { files: Arc::new(Mutex::new(HashMap::new())) }
    }
}

impl PromptAssetLoader for MemoryWorkspaceStore {
    fn read_asset(&self, path: &Path) -> std::io::Result<String> {
        self.read_file(path.to_str().unwrap()).map_err(|e| std::io::Error::other(e.to_string()))
    }

    fn asset_exists(&self, path: &Path) -> bool {
        let files = self.files.lock().unwrap();
        files.contains_key(path)
    }

    fn ensure_asset_dir(&self, _path: &Path) -> std::io::Result<()> {
        Ok(())
    }

    fn copy_asset(&self, _from: &Path, _to: &Path) -> std::io::Result<u64> {
        Ok(0)
    }
}

impl WorkspaceStore for MemoryWorkspaceStore {
    fn exists(&self) -> bool {
        let files = self.files.lock().unwrap();
        files.keys().any(|p| p.starts_with(JULES_DIR))
    }

    fn jlo_exists(&self) -> bool {
        let files = self.files.lock().unwrap();
        files.keys().any(|p| p.starts_with(JLO_DIR))
    }

    fn jules_path(&self) -> PathBuf {
        PathBuf::from(JULES_DIR)
    }

    fn jlo_path(&self) -> PathBuf {
        PathBuf::from(JLO_DIR)
    }

    fn create_structure(&self, scaffold_files: &[ScaffoldFile]) -> Result<(), AppError> {
        let mut files = self.files.lock().unwrap();
        for file in scaffold_files {
            files.insert(PathBuf::from(&file.path), file.content.as_bytes().to_vec());
        }
        Ok(())
    }

    fn write_version(&self, version: &str) -> Result<(), AppError> {
        self.write_file(&format!("{}/{}", JULES_DIR, VERSION_FILE), &format!("{}\n", version))
    }

    fn read_version(&self) -> Result<Option<String>, AppError> {
        let path = format!("{}/{}", JULES_DIR, VERSION_FILE);
        if let Ok(content) = self.read_file(&path) {
            Ok(Some(content.trim().to_string()))
        } else {
            Ok(None)
        }
    }

    fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError> {
        // Rudimentary implementation for testing
        Ok(vec![])
    }

    fn find_role_fuzzy(&self, _query: &str) -> Result<Option<DiscoveredRole>, AppError> {
        Ok(None)
    }

    fn role_path(&self, role: &DiscoveredRole) -> Option<PathBuf> {
        let path =
            jules::layer_roles_container(&self.jules_path(), role.layer).join(role.id.as_str());
        Some(path)
    }

    fn read_file(&self, path: &str) -> Result<String, AppError> {
        let files = self.files.lock().unwrap();
        let path = PathBuf::from(path);
        match files.get(&path) {
            Some(bytes) => {
                String::from_utf8(bytes.clone()).map_err(|e| AppError::AssetError(e.to_string()))
            }
            None => Err(AppError::from(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path.display()),
            ))),
        }
    }

    fn open_file(&self, path: &str) -> Result<Box<dyn std::io::Read>, AppError> {
        let files = self.files.lock().unwrap();
        let path_buf = PathBuf::from(path);
        match files.get(&path_buf) {
            Some(bytes) => Ok(Box::new(std::io::Cursor::new(bytes.clone()))),
            None => Err(AppError::from(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path_buf.display()),
            ))),
        }
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), AppError> {
        let mut files = self.files.lock().unwrap();
        files.insert(PathBuf::from(path), content.as_bytes().to_vec());
        Ok(())
    }

    fn remove_file(&self, path: &str) -> Result<(), AppError> {
        let mut files = self.files.lock().unwrap();
        files.remove(&PathBuf::from(path));
        Ok(())
    }

    fn list_dir(&self, path: &str) -> Result<Vec<PathBuf>, AppError> {
        let files = self.files.lock().unwrap();
        let path = PathBuf::from(path);
        let mut results = Vec::new();

        for key in files.keys() {
            if let Some(parent) = key.parent()
                && parent == path
            {
                results.push(key.clone());
            }
        }
        results.sort();
        Ok(results)
    }

    fn set_executable(&self, _path: &str) -> Result<(), AppError> {
        Ok(())
    }

    fn file_exists(&self, path: &str) -> bool {
        let files = self.files.lock().unwrap();
        let path_buf = PathBuf::from(path);
        if files.contains_key(&path_buf) {
            return true;
        }
        // Check if it is a directory (prefix of any file)
        files.keys().any(|k| k.starts_with(&path_buf) && k != &path_buf)
    }

    fn is_dir(&self, path: &str) -> bool {
        let files = self.files.lock().unwrap();
        let path_buf = PathBuf::from(path);

        if files.contains_key(&path_buf) {
            return false;
        }

        files.keys().any(|k| k.starts_with(&path_buf))
    }

    fn create_dir_all(&self, _path: &str) -> Result<(), AppError> {
        Ok(())
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    fn canonicalize(&self, path: &str) -> Result<PathBuf, AppError> {
        Ok(PathBuf::from(path))
    }
}
