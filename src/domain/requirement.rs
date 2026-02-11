use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::domain::AppError;
use crate::ports::WorkspaceStore;

/// Header fields for a requirement.
///
/// This struct represents the YAML schema for requirement files. All fields are
/// retained for schema fidelity even if not directly consumed by current callers.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RequirementHeader {
    /// Label for the requirement (e.g., bugs, feats, refacts).
    #[serde(default)]
    pub label: String,
    /// Whether the requirement requires deep analysis (planner) or implementation (implementer).
    #[serde(default)]
    pub requires_deep_analysis: bool,
}

impl RequirementHeader {
    /// Read requirement header from a file in the workspace.
    pub fn read(store: &impl WorkspaceStore, path: &Path) -> Result<Self, AppError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| AppError::Validation(format!("Invalid path: {}", path.display())))?;
        let content = store.read_file(path_str)?;
        let header: RequirementHeader = serde_yaml::from_str(&content).map_err(|e| {
            AppError::ParseError { what: path.display().to_string(), details: e.to_string() }
        })?;
        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AppError, IoErrorKind, PromptAssetLoader};
    use crate::ports::{DiscoveredRole, WorkspaceStore};
    use std::collections::HashMap;
    use std::path::PathBuf;

    struct MockWorkspaceStore {
        files: HashMap<String, String>,
    }

    impl MockWorkspaceStore {
        fn new() -> Self {
            Self { files: HashMap::new() }
        }

        fn add_file(&mut self, path: &str, content: &str) {
            self.files.insert(path.to_string(), content.to_string());
        }
    }

    // Minimal implementation of PromptAssetLoader for WorkspaceStore trait requirement
    impl PromptAssetLoader for MockWorkspaceStore {
        fn read_asset(&self, _path: &Path) -> std::io::Result<String> {
            unimplemented!()
        }
        fn asset_exists(&self, _path: &Path) -> bool {
            unimplemented!()
        }
        fn ensure_asset_dir(&self, _path: &Path) -> std::io::Result<()> {
            unimplemented!()
        }
        fn copy_asset(&self, _from: &Path, _to: &Path) -> std::io::Result<u64> {
            unimplemented!()
        }
    }

    impl WorkspaceStore for MockWorkspaceStore {
        fn read_file(&self, path: &str) -> Result<String, AppError> {
            self.files.get(path).cloned().ok_or_else(|| AppError::Io {
                message: "file not found".to_string(),
                kind: IoErrorKind::NotFound,
            })
        }

        // Unused methods
        fn exists(&self) -> bool {
            unimplemented!()
        }
        fn jlo_exists(&self) -> bool {
            unimplemented!()
        }
        fn jules_path(&self) -> PathBuf {
            unimplemented!()
        }
        fn jlo_path(&self) -> PathBuf {
            unimplemented!()
        }
        fn create_structure(
            &self,
            _scaffold_files: &[crate::ports::ScaffoldFile],
        ) -> Result<(), AppError> {
            unimplemented!()
        }
        fn write_version(&self, _version: &str) -> Result<(), AppError> {
            unimplemented!()
        }
        fn read_version(&self) -> Result<Option<String>, AppError> {
            unimplemented!()
        }
        fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError> {
            unimplemented!()
        }
        fn find_role_fuzzy(&self, _query: &str) -> Result<Option<DiscoveredRole>, AppError> {
            unimplemented!()
        }
        fn role_path(&self, _role: &DiscoveredRole) -> Option<PathBuf> {
            unimplemented!()
        }
        fn write_file(&self, _path: &str, _content: &str) -> Result<(), AppError> {
            unimplemented!()
        }
        fn remove_file(&self, _path: &str) -> Result<(), AppError> {
            unimplemented!()
        }
        fn list_dir(&self, _path: &str) -> Result<Vec<PathBuf>, AppError> {
            unimplemented!()
        }
        fn set_executable(&self, _path: &str) -> Result<(), AppError> {
            unimplemented!()
        }
        fn file_exists(&self, _path: &str) -> bool {
            unimplemented!()
        }
        fn is_dir(&self, _path: &str) -> bool {
            unimplemented!()
        }
        fn create_dir_all(&self, _path: &str) -> Result<(), AppError> {
            unimplemented!()
        }
        fn resolve_path(&self, _path: &str) -> PathBuf {
            unimplemented!()
        }
        fn canonicalize(&self, _path: &str) -> Result<PathBuf, AppError> {
            unimplemented!()
        }
    }

    #[test]
    fn read_requirement_header_success() {
        let mut store = MockWorkspaceStore::new();
        store.add_file("req.yml", "label: bugs\nrequires_deep_analysis: true");

        let header = RequirementHeader::read(&store, Path::new("req.yml")).unwrap();
        assert_eq!(header.label, "bugs");
        assert!(header.requires_deep_analysis);
    }

    #[test]
    fn read_requirement_header_default_values() {
        let mut store = MockWorkspaceStore::new();
        store.add_file("req.yml", "label: features"); // Missing requires_deep_analysis

        let header = RequirementHeader::read(&store, Path::new("req.yml")).unwrap();
        assert_eq!(header.label, "features");
        assert!(!header.requires_deep_analysis); // Default should be false
    }

    #[test]
    fn read_requirement_header_file_not_found() {
        let store = MockWorkspaceStore::new();
        let result = RequirementHeader::read(&store, Path::new("nonexistent.yml"));
        assert!(result.is_err());
    }

    #[test]
    fn read_requirement_header_parse_error() {
        let mut store = MockWorkspaceStore::new();
        store.add_file("req.yml", "invalid: [ yaml");
        let result = RequirementHeader::read(&store, Path::new("req.yml"));
        assert!(matches!(result, Err(AppError::ParseError { .. })));
    }
}
