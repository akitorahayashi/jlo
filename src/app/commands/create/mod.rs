//! Create command: explicit authoring of roles under `.jlo/`.

mod role;

use crate::app::AppContext;
use crate::domain::AppError;
use crate::ports::{RoleTemplateStore, WorkspaceStore};

/// Outcome of a create operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateOutcome {
    Role { layer: String, role: String },
}

pub(crate) fn role_relative_path(layer: &str, role: &str) -> std::path::PathBuf {
    std::path::Path::new("roles").join(layer).join("roles").join(role)
}

impl CreateOutcome {
    pub fn display_path(&self) -> String {
        let relative = match self {
            CreateOutcome::Role { layer, role } => role_relative_path(layer, role),
        };
        format!(".jlo/{}", relative.display())
    }

    pub fn entity_type(&self) -> &'static str {
        match self {
            CreateOutcome::Role { .. } => "role",
        }
    }
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
