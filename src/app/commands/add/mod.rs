//! Add command: register built-in roles in `.jlo/config.toml`.

mod role;

use crate::app::AppContext;
use crate::domain::AppError;
use crate::domain::PromptAssetLoader;
use crate::ports::{JloStore, JulesStore, RepositoryFilesystem, RoleTemplateStore};

/// Outcome of an add operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddOutcome {
    Role { layer: String, role: String },
}

impl AddOutcome {
    pub fn display_path(&self) -> String {
        match self {
            AddOutcome::Role { .. } => ".jlo/config.toml".to_string(),
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
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
    R: RoleTemplateStore,
{
    role::execute(ctx, layer, role)
}
