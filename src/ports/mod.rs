mod clipboard_writer;
mod role_template_store;
mod workspace_store;

pub use clipboard_writer::{ClipboardWriter, NoopClipboard};
pub use role_template_store::{RoleTemplateStore, ScaffoldFile};
pub use workspace_store::{DiscoveredRole, WorkspaceStore};
