//! Component catalog port definition.

use crate::domain::Component;

/// Trait for accessing the component catalog.
pub trait ComponentCatalogPort {
    /// Get a component by name.
    fn get(&self, name: &str) -> Option<&Component>;

    /// List all available components sorted by name.
    fn list_all(&self) -> Vec<&Component>;

    /// Get all component names.
    fn names(&self) -> Vec<&str>;
}
