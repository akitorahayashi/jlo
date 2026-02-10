//! Create role under `.jlo/roles/<layer>/<name>/`.

use crate::app::AppContext;
use crate::domain::identifiers::validation::validate_safe_path_component;
use crate::domain::{AppError, Layer};
use crate::ports::{RoleTemplateStore, WorkspaceStore};

use super::CreateOutcome;

pub fn execute<W, R>(
    ctx: &AppContext<W, R>,
    layer: &str,
    name: &str,
) -> Result<CreateOutcome, AppError>
where
    W: WorkspaceStore,
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

    if !validate_safe_path_component(name) {
        return Err(AppError::Validation(format!(
            "Invalid role name '{}'. Use alphanumeric characters, hyphens, or underscores.",
            name
        )));
    }

    let role_dir =
        ctx.workspace().jlo_path().join(super::role_relative_path(layer_enum.dir_name(), name));

    if role_dir.exists() {
        return Err(AppError::Validation(format!(
            "Role '{}' already exists in layer '{}' at {}",
            name,
            layer,
            role_dir.display()
        )));
    }

    // Seed with default role.yml from role templates
    let role_content = ctx.templates().generate_role_yaml(name, layer_enum);

    std::fs::create_dir_all(&role_dir)?;
    std::fs::write(role_dir.join("role.yml"), role_content)?;

    Ok(CreateOutcome::Role { layer: layer_enum.dir_name().to_string(), role: name.to_string() })
}
