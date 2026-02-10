//! Add command: install built-in roles under `.jlo/`.

mod role;

use crate::app::AppContext;
use crate::domain::AppError;
use crate::ports::{RoleTemplateStore, WorkspaceStore};

/// Outcome of an add operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddOutcome {
    Role { layer: String, role: String },
}

impl AddOutcome {
    pub fn display_path(&self) -> String {
        match self {
            AddOutcome::Role { layer, role } => {
                format!(".jlo/roles/{}/{}/", layer, role)
            }
        }
    }

    pub fn entity_type(&self) -> &'static str {
        match self {
            AddOutcome::Role { .. } => "role",
        }
    }
}

pub fn add_role<W, R>(
    ctx: &AppContext<W, R>,
    layer: &str,
    role: &str,
) -> Result<AddOutcome, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
{
    role::execute(ctx, layer, role)
}
