//! Create role under `.jlo/roles/<layer>/<name>/`.

use crate::app::AppContext;
use crate::domain::PromptAssetLoader;
use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{JloStore, JulesStore, RepositoryFilesystem, RoleTemplateStore};

use crate::app::commands::role_schedule::ensure_role_scheduled;

use super::RoleCreateOutcome;

pub fn execute<W, R>(
    ctx: &AppContext<W, R>,
    layer: &str,
    name: &str,
) -> Result<RoleCreateOutcome, AppError>
where
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
    R: RoleTemplateStore,
{
    if !ctx.repository().jlo_exists() {
        return Err(AppError::Validation(
            "repository is not initialized. Run 'jlo init' first.".to_string(),
        ));
    }

    let layer_enum = Layer::from_dir_name(layer)
        .ok_or_else(|| AppError::InvalidLayer { name: layer.to_string() })?;

    if layer_enum.is_single_role() {
        return Err(AppError::SingleRoleLayerTemplate(layer.to_string()));
    }

    let role_id = RoleId::new(name)?;
    let role_dir = format!(".jlo/roles/{}/{}", layer_enum.dir_name(), role_id.as_str());
    if ctx.repository().file_exists(&role_dir) {
        return Err(AppError::Validation(format!(
            "Role '{}' already exists in layer '{}' at {}",
            role_id.as_str(),
            layer_enum.dir_name(),
            role_dir
        )));
    }

    // Seed with default role.yml from role templates
    let role_content = ctx.templates().generate_role_yaml(role_id.as_str(), layer_enum);
    ctx.repository().create_dir_all(&role_dir)?;
    ctx.repository().write_file(&format!("{}/role.yml", role_dir), &role_content)?;

    ensure_role_scheduled(ctx.repository(), layer_enum, &role_id)?;

    Ok(RoleCreateOutcome::Role {
        layer: layer_enum.dir_name().to_string(),
        role: role_id.as_str().to_string(),
    })
}
