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

impl CreateOutcome {
    pub fn display_path(&self) -> String {
        match self {
            CreateOutcome::Role { layer, role } => {
                format!(".jlo/roles/{}/roles/{}", layer, role)
            }
            CreateOutcome::Workstream { name } => {
                format!(".jlo/workstreams/{}", name)
            }
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
