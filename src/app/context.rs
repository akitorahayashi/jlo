use crate::ports::{RoleTemplateStore, WorkspaceStore};

/// Application context holding dependencies for command execution.
pub struct AppContext<W: WorkspaceStore, R: RoleTemplateStore> {
    workspace: W,
    templates: R,
}

impl<W: WorkspaceStore, R: RoleTemplateStore> AppContext<W, R> {
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
