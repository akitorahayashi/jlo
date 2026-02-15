//! Repository-level filesystem operations.
//!
//! This port provides generic file I/O scoped to the repository root.
//! It does not own `.jlo/` or `.jules/` structure semantics — those
//! belong to domain (path catalog) and their respective store ports.

use std::path::PathBuf;

use crate::domain::AppError;

/// Port for low-level repository filesystem operations.
///
/// All `path` arguments are relative to the repository root.
/// Implementations must reject paths that escape the root boundary.
pub trait RepositoryFilesystem {
    /// Read a file as UTF-8 text.
    fn read_file(&self, path: &str) -> Result<String, AppError>;

    /// Write UTF-8 content to a file, creating parent directories as needed.
    fn write_file(&self, path: &str, content: &str) -> Result<(), AppError>;

    /// Remove a file. No-op if the file does not exist.
    fn remove_file(&self, path: &str) -> Result<(), AppError>;

    /// Remove a directory and all its contents. No-op if absent.
    fn remove_dir_all(&self, path: &str) -> Result<(), AppError>;

    /// List entries in a directory (returns paths relative to root).
    fn list_dir(&self, path: &str) -> Result<Vec<PathBuf>, AppError>;

    /// Set the executable bit on a file (Unix-only).
    fn set_executable(&self, path: &str) -> Result<(), AppError>;

    /// Check whether a file or directory exists.
    fn file_exists(&self, path: &str) -> bool;

    /// Check whether a path is a directory.
    fn is_dir(&self, path: &str) -> bool;

    /// Create a directory and all parent directories.
    fn create_dir_all(&self, path: &str) -> Result<(), AppError>;

    /// Resolve a relative path to an absolute path within the repository root.
    fn resolve_path(&self, path: &str) -> PathBuf;

    /// Canonicalize a path (resolve symlinks, produce absolute path).
    fn canonicalize(&self, path: &str) -> Result<PathBuf, AppError>;
}
