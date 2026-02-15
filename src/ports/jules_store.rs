//! `.jules/` runtime data-plane store operations.
//!
//! This port encapsulates domain-facing operations on the `.jules/` directory.
//! Path semantics are owned by `domain::workspace::paths::jules`;
//! this port owns only the I/O behavior.

use std::path::PathBuf;

use crate::domain::{AppError, PromptAssetLoader};
use crate::ports::ScaffoldFile;

/// Port for `.jules/` runtime data-plane store operations.
///
/// Extends `PromptAssetLoader` (defined in domain) so that any
/// `JulesStorePort` implementation also satisfies prompt assembly.
#[allow(dead_code)]
pub trait JulesStorePort: PromptAssetLoader {
    /// Check whether the `.jules/` directory exists.
    fn jules_exists(&self) -> bool;

    /// Absolute path to the `.jules/` directory.
    fn jules_path(&self) -> PathBuf;

    /// Create the `.jules/` directory structure from scaffold files.
    ///
    /// Writes all provided scaffold files and creates layer directories.
    fn create_structure(&self, scaffold_files: &[ScaffoldFile]) -> Result<(), AppError>;

    /// Write the `.jules/.jlo-version` version marker.
    fn jules_write_version(&self, version: &str) -> Result<(), AppError>;

    /// Read the `.jules/.jlo-version` version marker, if present.
    fn jules_read_version(&self) -> Result<Option<String>, AppError>;
}
