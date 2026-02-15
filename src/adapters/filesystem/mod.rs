//! Filesystem adapter implementations for store ports.
//!
//! Provides concrete adapters for `RepositoryFilesystemPort`, `JloStorePort`,
//! and `JulesStorePort`. All three are implemented on a single `FilesystemStore`
//! struct that owns the repository root path and enforces path-traversal safety.

#[allow(dead_code)] // Consumers will be migrated from FilesystemWorkspaceStore in later tasks.
mod jlo_store;
#[allow(dead_code)]
mod jules_store;
#[allow(dead_code)]
mod repository_filesystem;

use std::path::{Path, PathBuf};

use crate::domain::AppError;

/// Filesystem-backed store rooted at a repository directory.
///
/// Implements `RepositoryFilesystemPort`, `JloStorePort`, and `JulesStorePort`
/// on a single struct. Path operations are validated against the root to prevent
/// directory traversal.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FilesystemStore {
    root: PathBuf,
}

#[allow(dead_code)]
impl FilesystemStore {
    /// Create a store rooted at the given directory.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Create a store rooted at the current working directory.
    pub fn current() -> Result<Self, AppError> {
        let cwd = std::env::current_dir()?;
        Ok(Self::new(cwd))
    }

    /// The repository root.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

// ── Path safety ────────────────────────────────────────────────────────

impl FilesystemStore {
    /// Validates that a path (after logical normalization) is within the root.
    pub(crate) fn validate_path_within_root(&self, path: &Path) -> Result<(), AppError> {
        let full_path = if path.is_absolute() { path.to_path_buf() } else { self.root.join(path) };

        let normalized_path = normalize_path(&full_path);
        let normalized_root = normalize_path(&self.root);

        if !normalized_path.starts_with(&normalized_root) {
            return Err(AppError::PathTraversal(path.display().to_string()));
        }

        Ok(())
    }
}

/// Normalize path by resolving `.` and `..` components logically.
/// This does not access the filesystem.
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(std::path::Component::RootDir) = components.peek() {
        components.next();
        PathBuf::from("/")
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            std::path::Component::Prefix(..) => {
                ret.push(component.as_os_str());
            }
            std::path::Component::RootDir => {
                ret.push(component.as_os_str());
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                ret.pop();
            }
            std::path::Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    pub fn test_store() -> (TempDir, FilesystemStore) {
        let dir = TempDir::new().expect("failed to create temp dir");
        let store = FilesystemStore::new(dir.path().to_path_buf());
        (dir, store)
    }

    #[test]
    fn validate_path_prevents_traversal_with_nonexistent_components() {
        let (_dir, store) = test_store();

        // Simple escape
        let bad_path = "../result.txt";
        let result = store.validate_path_within_root(&store.root.join(bad_path));
        assert!(result.is_err(), "Should detect simple traversal");

        // Escape with non-existent intermediate
        let bad_path_complex = "nonexistent/../../outside_result.txt";
        let result = store.validate_path_within_root(&store.root.join(bad_path_complex));
        assert!(
            result.is_err(),
            "Should detect traversal even if 'nonexistent' components don't exist"
        );

        // Valid path with .. that stays inside
        let good_path_complex = "subdir/../result.txt";
        let result = store.validate_path_within_root(&store.root.join(good_path_complex));
        assert!(result.is_ok(), "Should allow .. that stays within root: {:?}", result.err());
    }
}
