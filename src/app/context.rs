use crate::domain::PromptAssetLoader;
use crate::ports::{JloStorePort, JulesStorePort, RepositoryFilesystemPort, RoleTemplateStore};

/// Application context holding dependencies for command execution.
pub struct AppContext<W, R>
where
    W: RepositoryFilesystemPort + JloStorePort + JulesStorePort + PromptAssetLoader,
    R: RoleTemplateStore,
{
    workspace: W,
    templates: R,
}

impl<W, R> AppContext<W, R>
where
    W: RepositoryFilesystemPort + JloStorePort + JulesStorePort + PromptAssetLoader,
    R: RoleTemplateStore,
{
    /// Create a new application context.
    pub fn new(workspace: W, templates: R) -> Self {
        Self { workspace, templates }
    }

    /// Get a reference to the workspace store.
    pub fn workspace(&self) -> &W {
        &self.workspace
    }

    /// Get a reference to the role template store.
    pub fn templates(&self) -> &R {
        &self.templates
    }
}
