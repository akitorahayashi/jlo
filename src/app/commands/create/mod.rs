//! Create command: explicit authoring of roles and workstreams under `.jlo/`.

mod role;
mod workstream;

use crate::app::AppContext;
use crate::domain::AppError;
use crate::ports::{RoleTemplateStore, WorkspaceStore};

/// Outcome of a create operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateOutcome {
    Role { layer: String, role: String },
    Workstream { name: String },
}

pub(crate) fn role_relative_path(layer: &str, role: &str) -> std::path::PathBuf {
    std::path::Path::new("roles").join(layer).join("roles").join(role)
}

pub(crate) fn workstream_relative_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new("workstreams").join(name)
}

impl CreateOutcome {
    pub fn display_path(&self) -> String {
        let relative = match self {
            CreateOutcome::Role { layer, role } => role_relative_path(layer, role),
            CreateOutcome::Workstream { name } => workstream_relative_path(name),
        };
        format!(".jlo/{}", relative.display())
    }

    pub fn entity_type(&self) -> &'static str {
        match self {
            CreateOutcome::Role { .. } => "role",
            CreateOutcome::Workstream { .. } => "workstream",
        }
    }
}

/// Create a new workstream under `.jlo/workstreams/<name>/`.
pub fn create_workstream<W, R>(
    ctx: &AppContext<W, R>,
    name: &str,
) -> Result<CreateOutcome, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
{
    workstream::execute(ctx, name)
}

/// Create a new role under `.jlo/roles/<layer>/roles/<name>/`.
pub fn create_role<W, R>(
    ctx: &AppContext<W, R>,
    layer: &str,
    name: &str,
) -> Result<CreateOutcome, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
{
    role::execute(ctx, layer, name)
}
