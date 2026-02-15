//! Legacy `WorkspaceStore` supertrait — migration bridge.
//!
//! `WorkspaceStore` is now a blanket supertrait over the three explicit ports:
//! `RepositoryFilesystemPort`, `JloStorePort`, and `JulesStorePort`.
//! Any type implementing all three ports automatically implements `WorkspaceStore`.
//!
//! **This trait is deprecated.** New code should depend on the specific ports.
//! It will be deleted once all consumers are migrated.

use crate::domain::{Layer, PromptAssetLoader, RoleId};

/// A discovered role with its layer and ID.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DiscoveredRole {
    pub layer: Layer,
    pub id: RoleId,
}

use super::{JloStorePort, JulesStorePort, RepositoryFilesystemPort};

/// Deprecated bridge trait. New code should depend on specific ports.
pub trait WorkspaceStore:
    RepositoryFilesystemPort + JloStorePort + JulesStorePort + PromptAssetLoader
{
}

/// Blanket impl: any type satisfying the three ports automatically is a `WorkspaceStore`.
impl<T> WorkspaceStore for T where
    T: RepositoryFilesystemPort + JloStorePort + JulesStorePort + PromptAssetLoader
{
}
