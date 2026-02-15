use crate::domain::PromptAssetLoader;
use crate::ports::{JloStore, JulesStore, RepositoryFilesystem, RoleTemplateStore};

/// Application context holding dependencies for command execution.
pub struct AppContext<W, R>
where
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
    R: RoleTemplateStore,
{
    repository: W,
    templates: R,
}

impl<W, R> AppContext<W, R>
where
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
    R: RoleTemplateStore,
{
    /// Create a new application context.
    pub fn new(repository: W, templates: R) -> Self {
        Self { repository, templates }
    }

    /// Get a reference to the repository adapter.
    pub fn repository(&self) -> &W {
        &self.repository
    }

    /// Get a reference to the role template store.
    pub fn templates(&self) -> &R {
        &self.templates
    }
}
