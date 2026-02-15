//! Add a builtin role under `.jlo/roles/<layer>/<name>/`.

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
    let entry =
        catalog.iter().find(|entry| entry.matches(layer_enum, &role_id)).ok_or_else(|| {
            let available: Vec<String> = catalog
                .iter()
                .filter(|entry| entry.layer == layer_enum)
                .map(|entry| entry.name.as_str().to_string())
                .collect();
            AppError::Validation(format!(
                "Builtin role '{}' not found in layer '{}'. Available: {}",
                role_id.as_str(),
                layer_enum.dir_name(),
                available.join(", ")
            ))
        })?;

    let role_dir = crate::domain::roles::paths::role_dir(
        &ctx.repository().resolve_path(""),
        layer_enum,
        role_id.as_str(),
    );

    if role_dir.exists() {
        return Err(AppError::RoleExists {
            role: role_id.as_str().to_string(),
            layer: layer_enum.dir_name().to_string(),
        });
    }

    let role_content = ctx.templates().builtin_role_content(&entry.path)?;

    std::fs::create_dir_all(&role_dir)?;
    std::fs::write(role_dir.join("role.yml"), role_content)?;

    ensure_role_scheduled(ctx.repository(), layer_enum, &role_id)?;

    Ok(AddOutcome::Role {
        layer: layer_enum.dir_name().to_string(),
        role: role_id.as_str().to_string(),
    })
}
