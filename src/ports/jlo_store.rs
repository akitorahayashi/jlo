//! `.jlo/` control-plane store operations.
//!
//! This port encapsulates domain-facing operations on the `.jlo/` directory.
//! Path semantics (which files live where) are owned by `domain::workspace::paths::jlo`;
//! this port owns only the I/O behavior.

use std::path::PathBuf;

use crate::domain::{AppError, Layer, RoleId};

/// A discovered role with its layer and ID.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DiscoveredRole {
    pub layer: Layer,
    pub id: RoleId,
}

/// Port for `.jlo/` control-plane store operations.
#[allow(dead_code)]
pub trait JloStorePort {
    /// Check whether the `.jlo/` directory exists.
    fn jlo_exists(&self) -> bool;

    /// Absolute path to the `.jlo/` directory.
    fn jlo_path(&self) -> PathBuf;

    /// Write the `.jlo/.jlo-version` version pin.
    fn jlo_write_version(&self, version: &str) -> Result<(), AppError>;

    /// Read the `.jlo/.jlo-version` version pin, if present.
    fn jlo_read_version(&self) -> Result<Option<String>, AppError>;

    /// Discover all roles with a valid `role.yml` across multi-role layers.
    fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError>;

    /// Find a role by exact match, `layer/role` format, or unique prefix.
    fn find_role_fuzzy(&self, query: &str) -> Result<Option<DiscoveredRole>, AppError>;

    /// Absolute path to a discovered role's directory, if it exists.
    fn role_path(&self, role: &DiscoveredRole) -> Option<PathBuf>;

    /// Write a role definition file at `.jlo/roles/<layer>/<role>/role.yml`.
    fn write_role(&self, layer: Layer, role_id: &str, content: &str) -> Result<(), AppError>;
}
