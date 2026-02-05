use crate::ports::{RoleTemplatePort, WorkspacePort};

/// Application context holding dependencies for command execution.
pub struct AppContext<W: WorkspacePort, R: RoleTemplatePort> {
    workspace: W,
    templates: R,
}

impl<W: WorkspacePort, R: RoleTemplatePort> AppContext<W, R> {
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
