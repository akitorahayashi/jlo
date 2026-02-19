//! Delete role under `.jlo/roles/<layer>/<name>/` and unschedule it.

use crate::app::AppContext;
use crate::domain::PromptAssetLoader;
use crate::domain::{AppError, Layer, RoleError, RoleId};
use crate::ports::{JloStore, JulesStore, RepositoryFilesystem, RoleTemplateStore};

use super::schedule::{ensure_role_scheduled, remove_role_scheduled};

use super::RoleDeleteOutcome;

pub fn execute<W, R>(
    ctx: &AppContext<W, R>,
    layer: &str,
    role: &str,
) -> Result<RoleDeleteOutcome, AppError>
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
        .ok_or_else(|| RoleError::InvalidLayer { name: layer.to_string() })?;
    if layer_enum.is_single_role() {
        return Err(RoleError::SingleRoleLayerTemplate(layer_enum.dir_name().to_string()).into());
    }

    let role_id = RoleId::new(role)?;
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
    let role_yml = crate::domain::roles::paths::role_yml(root, layer_enum, role_id.as_str());
    let role_yml_relative = role_yml.strip_prefix(root).unwrap_or(&role_yml);
    let role_yml_str = role_yml_relative.to_str().ok_or_else(|| {
        AppError::InvalidPath(format!(
            "Role file path contains invalid unicode: {}",
            role_yml_relative.display()
        ))
    })?;

    if !ctx.repository().file_exists(role_yml_str) {
        return Err(RoleError::NotFound(format!(
            "{}/{} (missing {})",
            layer_enum.dir_name(),
            role_id.as_str(),
            role_yml_str
        ))
        .into());
    }

    let removed = remove_role_scheduled(ctx.repository(), layer_enum, &role_id)?;
    if !removed {
        return Err(RoleError::NotInConfig {
            role: role_id.as_str().to_string(),
            layer: layer_enum.dir_name().to_string(),
        }
        .into());
    }

    if let Err(remove_err) = ctx.repository().remove_dir_all(role_dir_str) {
        match ensure_role_scheduled(ctx.repository(), layer_enum, &role_id) {
            Ok(_) => return Err(remove_err),
            Err(rollback_err) => {
                return Err(AppError::Validation(format!(
                    "Failed to delete role directory '{}': {}. Failed to restore schedule entry for '{}': {}",
                    role_dir_str,
                    remove_err,
                    role_id.as_str(),
                    rollback_err
                )));
            }
        }
    }

    Ok(RoleDeleteOutcome::Role {
        layer: layer_enum.dir_name().to_string(),
        role: role_id.as_str().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::catalogs::EmbeddedRoleTemplateStore;
    use crate::domain::RoleError;
    use crate::ports::RepositoryFilesystem;
    use crate::testing::TestStore;

    fn context(store: TestStore) -> crate::app::AppContext<TestStore, EmbeddedRoleTemplateStore> {
        crate::app::AppContext::new(store, EmbeddedRoleTemplateStore::new())
    }

    #[test]
    fn delete_role_removes_directory_and_schedule_entry() {
        let repository = TestStore::new()
            .with_exists(true)
            .with_file(
                ".jlo/config.toml",
                r#"[run]
jlo_target_branch = "target_branch"
jules_worker_branch = "worker_branch"

[observers]
roles = [
  { name = "taxonomy", enabled = true },
]
"#,
            )
            .with_file(
                ".jlo/roles/observers/taxonomy/role.yml",
                "role: taxonomy\nlayer: observers\n",
            );
        let ctx = context(repository.clone());

        let outcome = execute(&ctx, "observers", "taxonomy").expect("delete should succeed");
        assert_eq!(outcome.entity_type(), "role");
        assert_eq!(outcome.display_path(), ".jlo/roles/observers/taxonomy");
        assert!(!repository.file_exists(".jlo/roles/observers/taxonomy/role.yml"));

        let config = repository.read_file(".jlo/config.toml").expect("config should exist");
        assert!(!config.contains("taxonomy"));
    }

    #[test]
    fn delete_role_fails_when_not_in_schedule() {
        let repository = TestStore::new()
            .with_exists(true)
            .with_file(
                ".jlo/config.toml",
                r#"[run]
jlo_target_branch = "target_branch"
jules_worker_branch = "worker_branch"

[observers]
roles = [
  { name = "consistency", enabled = true },
]
"#,
            )
            .with_file(
                ".jlo/roles/observers/taxonomy/role.yml",
                "role: taxonomy\nlayer: observers\n",
            );
        let ctx = context(repository);

        let err = execute(&ctx, "observers", "taxonomy").expect_err("delete should fail");
        assert!(matches!(err, AppError::Role(RoleError::NotInConfig { .. })));
    }
}
