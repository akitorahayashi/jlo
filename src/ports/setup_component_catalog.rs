//! Setup component catalog port definition.

use crate::domain::SetupComponent;

/// Trait for accessing the setup component catalog.
pub trait SetupComponentCatalog {
    /// Get a component by name.
    fn get(&self, name: &str) -> Option<&SetupComponent>;

    /// List all available components sorted by name.
    fn list_all(&self) -> Vec<&SetupComponent>;

    /// Get all component names.
    fn names(&self) -> Vec<&str>;
}
