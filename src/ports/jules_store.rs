//! `.jules/` runtime data-plane store operations.
//!
//! This port encapsulates domain-facing operations on the `.jules/` directory.
//! Path semantics are owned by `domain::workspace::paths::jules`;
//! this port owns only the I/O behavior.

use std::path::Path;

use crate::domain::AppError;
use crate::ports::ScaffoldFile;

/// Port for `.jules/` runtime data-plane store operations.
pub trait JulesStorePort: PromptAssetLoaderPort {
    /// Check whether the `.jules/` directory exists.
    fn exists(&self) -> bool;

    /// Create the `.jules/` directory structure from scaffold files.
    ///
    /// Writes all provided scaffold files and creates layer directories.
    fn create_structure(&self, scaffold_files: &[ScaffoldFile]) -> Result<(), AppError>;

    /// Write the `.jules/.jlo-version` version marker.
    fn write_version(&self, version: &str) -> Result<(), AppError>;

    /// Read the `.jules/.jlo-version` version marker, if present.
    fn read_version(&self) -> Result<Option<String>, AppError>;
}

/// Asset loading abstraction used by prompt assembly.
///
/// Extracted as a port-level supertrait so that `JulesStorePort`
/// implementations also satisfy prompt assembly requirements.
pub trait PromptAssetLoaderPort {
    /// Read an asset file by absolute path.
    fn read_asset(&self, path: &Path) -> std::io::Result<String>;

    /// Check whether an asset file exists at the absolute path.
    fn asset_exists(&self, path: &Path) -> bool;

    /// Ensure the directory for an asset path exists.
    fn ensure_asset_dir(&self, path: &Path) -> std::io::Result<()>;

    /// Copy an asset file from one absolute path to another.
    fn copy_asset(&self, from: &Path, to: &Path) -> std::io::Result<u64>;
}
