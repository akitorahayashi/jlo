//! Test double for `JulesStorePort` and `PromptAssetLoader`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::domain::{AppError, PromptAssetLoader};
use crate::ports::{JulesStorePort, ScaffoldFile};

use super::test_files::TestFiles;

/// In-memory implementation of `JulesStorePort` + `PromptAssetLoader` for unit tests.
///
/// Holds `.jules/`-scoped state (existence, version, structure creation)
/// independently from `.jlo/`-scoped state in `MockJloStore`.
#[derive(Clone, Debug)]
pub struct MockJulesStore {
    files: TestFiles,
    pub exists: Arc<Mutex<bool>>,
    pub version: Arc<Mutex<Option<String>>>,
    pub created_structure: Arc<Mutex<bool>>,
}

#[allow(dead_code)]
impl MockJulesStore {
    pub fn new(files: TestFiles) -> Self {
        Self {
            files,
            exists: Arc::new(Mutex::new(false)),
            version: Arc::new(Mutex::new(None)),
            created_structure: Arc::new(Mutex::new(false)),
        }
    }

    pub fn with_exists(self, exists: bool) -> Self {
        *self.exists.lock().unwrap() = exists;
        self
    }
}

impl PromptAssetLoader for MockJulesStore {
    fn read_asset(&self, path: &Path) -> std::io::Result<String> {
        let path_str = path.to_string_lossy().to_string();
        self.files
            .files
            .lock()
            .unwrap()
            .get(&path_str)
            .cloned()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Mock file not found"))
    }

    fn asset_exists(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_string();
        self.files.files.lock().unwrap().contains_key(&path_str)
    }

    fn ensure_asset_dir(&self, _path: &Path) -> std::io::Result<()> {
        Ok(())
    }

    fn copy_asset(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        let from_str = from.to_string_lossy().to_string();
        let to_str = to.to_string_lossy().to_string();
        let mut files = self.files.files.lock().unwrap();
        let content = files.get(&from_str).cloned().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Source file not found")
        })?;
        files.insert(to_str, content.clone());
        Ok(content.len() as u64)
    }
}

impl JulesStorePort for MockJulesStore {
    fn jules_exists(&self) -> bool {
        *self.exists.lock().unwrap()
    }

    fn jules_path(&self) -> PathBuf {
        PathBuf::from(".jules")
    }

    fn create_structure(&self, _scaffold_files: &[ScaffoldFile]) -> Result<(), AppError> {
        *self.created_structure.lock().unwrap() = true;
        *self.exists.lock().unwrap() = true;
        Ok(())
    }

    fn jules_write_version(&self, version: &str) -> Result<(), AppError> {
        *self.version.lock().unwrap() = Some(version.to_string());
        Ok(())
    }

    fn jules_read_version(&self) -> Result<Option<String>, AppError> {
        Ok(self.version.lock().unwrap().clone())
    }
}
