//! Register a builtin role in `.jlo/config.toml`.

use crate::app::AppContext;
use crate::domain::PromptAssetLoader;
use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{JloStore, JulesStore, RepositoryFilesystem, RoleTemplateStore};

use crate::app::commands::role_schedule::ensure_role_scheduled;

use super::AddOutcome;

pub fn execute<W, R>(
    ctx: &AppContext<W, R>,
    layer: &str,
    role: &str,
) -> Result<AddOutcome, AppError>
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

    Ok(AddOutcome::Role {
        layer: layer_enum.dir_name().to_string(),
        role: role_id.as_str().to_string(),
    })
}
