//! Create role under `.jlo/roles/<layer>/<name>/`.

use crate::app::AppContext;
use crate::domain::PromptAssetLoader;
use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{JloStorePort, JulesStorePort, RepositoryFilesystemPort, RoleTemplateStore};

use super::CreateOutcome;
use crate::app::commands::role_schedule::ensure_role_scheduled;

pub fn execute<W, R>(
    ctx: &AppContext<W, R>,
    layer: &str,
    name: &str,
) -> Result<CreateOutcome, AppError>
where
    W: RepositoryFilesystemPort + JloStorePort + JulesStorePort + PromptAssetLoader,
    R: RoleTemplateStore,
{
    if !ctx.workspace().jlo_exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let layer_enum = Layer::from_dir_name(layer)
        .ok_or_else(|| AppError::InvalidLayer { name: layer.to_string() })?;

    if layer_enum.is_single_role() {
        return Err(AppError::SingleRoleLayerTemplate(layer.to_string()));
    }

    let role_id = RoleId::new(name)?;

    let role_dir = ctx
        .workspace()
        .jlo_path()
        .join(super::role_relative_path(layer_enum.dir_name(), role_id.as_str()));

    if role_dir.exists() {
        return Err(AppError::Validation(format!(
            "Role '{}' already exists in layer '{}' at {}",
            name,
            layer,
            role_dir.display()
        )));
    }

    // Seed with default role.yml from role templates
    let role_content = ctx.templates().generate_role_yaml(role_id.as_str(), layer_enum);

    std::fs::create_dir_all(&role_dir)?;
    std::fs::write(role_dir.join("role.yml"), role_content)?;

    ensure_role_scheduled(ctx.workspace(), layer_enum, &role_id)?;

    Ok(CreateOutcome::Role {
        layer: layer_enum.dir_name().to_string(),
        role: role_id.as_str().to_string(),
    })
}
