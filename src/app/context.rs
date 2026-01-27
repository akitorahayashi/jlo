use crate::ports::{ClipboardWriter, RoleTemplateStore, WorkspaceStore};

/// Application context holding dependencies for command execution.
pub struct AppContext<W: WorkspaceStore, R: RoleTemplateStore, C: ClipboardWriter> {
    workspace: W,
    templates: R,
    clipboard: C,
}

impl<W: WorkspaceStore, R: RoleTemplateStore, C: ClipboardWriter> AppContext<W, R, C> {
    /// Create a new application context.
    pub fn new(workspace: W, templates: R, clipboard: C) -> Self {
        Self { workspace, templates, clipboard }
    }

    /// Get a reference to the workspace store.
    pub fn workspace(&self) -> &W {
        &self.workspace
    }

    /// Get a reference to the role template store.
    pub fn templates(&self) -> &R {
        &self.templates
    }

    /// Get a mutable reference to the clipboard writer.
    pub fn clipboard_mut(&mut self) -> &mut C {
        &mut self.clipboard
    }
}
