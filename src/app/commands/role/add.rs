//! Register a builtin role in `.jlo/config.toml`.

use crate::app::AppContext;
use crate::domain::PromptAssetLoader;
use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{JloStore, JulesStore, RepositoryFilesystem, RoleTemplateStore};

use crate::app::commands::role_schedule::ensure_role_scheduled;

use super::RoleAddOutcome;

pub fn execute<W, R>(
    ctx: &AppContext<W, R>,
    layer: &str,
    role: &str,
) -> Result<RoleAddOutcome, AppError>
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

    let role_id = RoleId::new(role)?;

    let catalog = ctx.templates().builtin_role_catalog()?;
    if catalog.iter().all(|entry| !entry.matches(layer_enum, &role_id)) {
        let available: Vec<String> = catalog
            .iter()
            .filter(|entry| entry.layer == layer_enum)
            .map(|entry| entry.name.as_str().to_string())
            .collect();
        return Err(AppError::Validation(format!(
            "Builtin role '{}' not found in layer '{}'. Available: {}",
            role_id.as_str(),
            layer_enum.dir_name(),
            available.join(", ")
        )));
    }

    let inserted = ensure_role_scheduled(ctx.repository(), layer_enum, &role_id)?;
    if !inserted {
        return Err(AppError::RoleExists {
            role: role_id.as_str().to_string(),
            layer: layer_enum.dir_name().to_string(),
        });
    }

    let jlo_path = ctx.repository().jlo_path();
    let root = jlo_path.parent().ok_or_else(|| {
        AppError::InvalidPath(format!("Invalid .jlo path (missing parent): {}", jlo_path.display()))
    })?;
    let role_path = crate::domain::roles::paths::role_yml(root, layer_enum, role_id.as_str());
    let role_path_str = role_path.to_str().ok_or_else(|| {
        AppError::InvalidPath(format!(
            "Role path contains invalid unicode: {}",
            role_path.display()
        ))
    })?;
    if !ctx.repository().file_exists(role_path_str) {
        let content = ctx.templates().builtin_role_content(layer_enum, role_id.as_str())?;
        ctx.repository().write_role(layer_enum, role_id.as_str(), &content)?;
    }

    Ok(RoleAddOutcome::Role {
        layer: layer_enum.dir_name().to_string(),
        role: role_id.as_str().to_string(),
    })
}
