//! Create role under `.jlo/roles/<layer>/<name>/`.

use crate::app::AppContext;
use crate::domain::PromptAssetLoader;
use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{JloStore, JulesStore, RepositoryFilesystem, RoleTemplateStore};

use super::schedule::ensure_role_scheduled;

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
    let jlo_path = ctx.repository().jlo_path();
    let root = jlo_path.parent().ok_or_else(|| {
        AppError::InvalidPath(format!("Invalid .jlo path (missing parent): {}", jlo_path.display()))
    })?;
    let role_dir = crate::domain::roles::paths::role_dir(root, layer_enum, role_id.as_str());
    let role_dir_relative = role_dir.strip_prefix(root).unwrap_or(&role_dir);
    let role_dir_str = role_dir_relative.to_str().ok_or_else(|| {
        AppError::InvalidPath(format!(
            "Role directory path contains invalid unicode: {}",
            role_dir_relative.display()
        ))
    })?;

    if ctx.repository().file_exists(role_dir_str) {
        return Err(AppError::Validation(format!(
            "Role '{}' already exists in layer '{}' at {}",
            role_id.as_str(),
            layer_enum.dir_name(),
            role_dir_str
        )));
    }

    // Seed with default role.yml from role templates
    let role_content = ctx.templates().generate_role_yaml(role_id.as_str(), layer_enum);
    let role_yml = crate::domain::roles::paths::role_yml(root, layer_enum, role_id.as_str());
    let role_yml_relative = role_yml.strip_prefix(root).unwrap_or(&role_yml);
    let role_yml_str = role_yml_relative.to_str().ok_or_else(|| {
        AppError::InvalidPath(format!(
            "Role file path contains invalid unicode: {}",
            role_yml_relative.display()
        ))
    })?;
    ctx.repository().create_dir_all(role_dir_str)?;
    ctx.repository().write_file(role_yml_str, &role_content)?;

    ensure_role_scheduled(ctx.repository(), layer_enum, &role_id)?;

    Ok(RoleCreateOutcome::Role {
        layer: layer_enum.dir_name().to_string(),
        role: role_id.as_str().to_string(),
    })
}
