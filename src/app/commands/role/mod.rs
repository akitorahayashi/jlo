//! Role lifecycle commands under `.jlo/`.

mod add;
mod create;
mod delete;
mod schedule;

use crate::app::AppContext;
use crate::domain::AppError;
use crate::domain::PromptAssetLoader;
use crate::ports::{JloStore, JulesStore, RepositoryFilesystem, RoleTemplateStore};

/// Outcome of a role add operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleAddOutcome {
    Role { layer: String, role: String },
}

impl RoleAddOutcome {
    pub fn display_path(&self) -> String {
        match self {
            RoleAddOutcome::Role { .. } => ".jlo/config.toml".to_string(),
        }
    }

    pub fn entity_type(&self) -> &'static str {
        match self {
            RoleAddOutcome::Role { .. } => "role",
        }
    }
}

/// Outcome of a role create operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleCreateOutcome {
    Role { layer: String, role: String },
}

pub(crate) fn role_relative_path(layer: &str, role: &str) -> std::path::PathBuf {
    std::path::Path::new("roles").join(layer).join(role)
}

impl RoleCreateOutcome {
    pub fn display_path(&self) -> String {
        let relative = match self {
            RoleCreateOutcome::Role { layer, role } => role_relative_path(layer, role),
        };
        format!(".jlo/{}", relative.display())
    }

    pub fn entity_type(&self) -> &'static str {
        match self {
            RoleCreateOutcome::Role { .. } => "role",
        }
    }
}

/// Outcome of a role delete operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleDeleteOutcome {
    Role { layer: String, role: String },
}

impl RoleDeleteOutcome {
    pub fn display_path(&self) -> String {
        let relative = match self {
            RoleDeleteOutcome::Role { layer, role } => role_relative_path(layer, role),
        };
        format!(".jlo/{}", relative.display())
    }

    pub fn entity_type(&self) -> &'static str {
        match self {
            RoleDeleteOutcome::Role { .. } => "role",
        }
    }
}

/// Register a built-in role in `.jlo/config.toml`.
pub fn add_role<W, R>(
    ctx: &AppContext<W, R>,
    layer: &str,
    role: &str,
) -> Result<RoleAddOutcome, AppError>
where
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
    R: RoleTemplateStore,
{
    add::execute(ctx, layer, role)
}

/// Create a new role under `.jlo/roles/<layer>/<name>/`.
pub fn create_role<W, R>(
    ctx: &AppContext<W, R>,
    layer: &str,
    name: &str,
) -> Result<RoleCreateOutcome, AppError>
where
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
    R: RoleTemplateStore,
{
    create::execute(ctx, layer, name)
}

/// Delete role directory and schedule entry.
pub fn delete_role<W, R>(
    ctx: &AppContext<W, R>,
    layer: &str,
    role: &str,
) -> Result<RoleDeleteOutcome, AppError>
where
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
    R: RoleTemplateStore,
{
    delete::execute(ctx, layer, role)
}
