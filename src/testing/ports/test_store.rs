//! Composite test store assembling port-scoped doubles.
//!
//! Production functions accept `&W` where `W` implements multiple port traits.
//! `TestStore` composes `MockRepositoryFs`, `MockJloStore`, and `MockJulesStore`
//! into a single value that satisfies these combined bounds.

use std::path::{Path, PathBuf};

use crate::domain::{AppError, Layer, PromptAssetLoader};
use crate::ports::{DiscoveredRole, JloStore, JulesStore, RepositoryFilesystem, ScaffoldFile};

use super::test_files::TestFiles;
use super::test_jlo_store::MockJloStore;
use super::test_jules_store::MockJulesStore;
use super::test_repository_fs::MockRepositoryFs;

/// Composite test double that delegates to port-scoped doubles.
///
/// Constructed from `TestFiles` with independent jlo/jules state.
/// Use [`TestStore::new`] for quick setup, or assemble from individual
/// port doubles for fine-grained control.
#[derive(Clone, Debug)]
pub struct TestStore {
    pub fs: MockRepositoryFs,
    pub jlo: MockJloStore,
    pub jules: MockJulesStore,
}

impl TestStore {
    /// Create a new `TestStore` with empty files and default state.
    pub fn new() -> Self {
        let files = TestFiles::new();
        Self {
            fs: MockRepositoryFs::new(files.clone()),
            jlo: MockJloStore::new(files.clone()),
            jules: MockJulesStore::new(files),
        }
    }

    /// Set both jlo and jules existence. Convenience for the common case
    /// where `.jlo/` and `.jules/` co-exist.
    pub fn with_exists(self, exists: bool) -> Self {
        *self.jlo.exists.lock().unwrap() = exists;
        *self.jules.exists.lock().unwrap() = exists;
        self
    }

    /// Seed a file into the shared backing store.
    pub fn with_file(self, path: &str, content: &str) -> Self {
        self.fs.write_file(path, content).expect("test file seeding should not fail");
        self
    }

    /// Add a role to the jlo store.
    #[allow(dead_code)]
    pub fn add_role(&self, layer: Layer, role_id: &str) {
        self.jlo.add_role(layer, role_id);
    }
}

impl Default for TestStore {
    fn default() -> Self {
        Self::new()
    }
}

// --- Delegate RepositoryFilesystem to self.fs ---

impl RepositoryFilesystem for TestStore {
    fn read_file(&self, path: &str) -> Result<String, AppError> {
        self.fs.read_file(path)
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), AppError> {
        self.fs.write_file(path, content)
    }

    fn remove_file(&self, path: &str) -> Result<(), AppError> {
        self.fs.remove_file(path)
    }

    fn remove_dir_all(&self, path: &str) -> Result<(), AppError> {
        self.fs.remove_dir_all(path)
    }

    fn list_dir(&self, path: &str) -> Result<Vec<PathBuf>, AppError> {
        self.fs.list_dir(path)
    }

    fn set_executable(&self, path: &str) -> Result<(), AppError> {
        self.fs.set_executable(path)
    }

    fn file_exists(&self, path: &str) -> bool {
        self.fs.file_exists(path)
    }

    fn is_dir(&self, path: &str) -> bool {
        self.fs.is_dir(path)
    }

    fn create_dir_all(&self, path: &str) -> Result<(), AppError> {
        self.fs.create_dir_all(path)
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        self.fs.resolve_path(path)
    }

    fn canonicalize(&self, path: &str) -> Result<PathBuf, AppError> {
        self.fs.canonicalize(path)
    }
}

// --- Delegate JloStore to self.jlo ---

impl JloStore for TestStore {
    fn jlo_exists(&self) -> bool {
        self.jlo.jlo_exists()
    }

    fn jlo_path(&self) -> PathBuf {
        self.jlo.jlo_path()
    }

    fn jlo_write_version(&self, version: &str) -> Result<(), AppError> {
        self.jlo.jlo_write_version(version)
    }

    fn jlo_read_version(&self) -> Result<Option<String>, AppError> {
        self.jlo.jlo_read_version()
    }

    fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError> {
        self.jlo.discover_roles()
    }

    fn find_role_fuzzy(&self, query: &str) -> Result<Option<DiscoveredRole>, AppError> {
        self.jlo.find_role_fuzzy(query)
    }

    fn role_path(&self, role: &DiscoveredRole) -> Option<PathBuf> {
        self.jlo.role_path(role)
    }

    fn write_role(&self, layer: Layer, role_id: &str, content: &str) -> Result<(), AppError> {
        self.jlo.write_role(layer, role_id, content)
    }
}

// --- Delegate JulesStore to self.jules ---

impl JulesStore for TestStore {
    fn jules_exists(&self) -> bool {
        self.jules.jules_exists()
    }

    fn jules_path(&self) -> PathBuf {
        self.jules.jules_path()
    }

    fn create_structure(&self, scaffold_files: &[ScaffoldFile]) -> Result<(), AppError> {
        self.jules.create_structure(scaffold_files)
    }

    fn jules_write_version(&self, version: &str) -> Result<(), AppError> {
        self.jules.jules_write_version(version)
    }

    fn jules_read_version(&self) -> Result<Option<String>, AppError> {
        self.jules.jules_read_version()
    }
}

// --- Delegate PromptAssetLoader to self.jules ---

impl PromptAssetLoader for TestStore {
    fn read_asset(&self, path: &Path) -> std::io::Result<String> {
        self.jules.read_asset(path)
    }

    fn asset_exists(&self, path: &Path) -> bool {
        self.jules.asset_exists(path)
    }

    fn ensure_asset_dir(&self, path: &Path) -> std::io::Result<()> {
        self.jules.ensure_asset_dir(path)
    }

    fn copy_asset(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        self.jules.copy_asset(from, to)
    }
}
