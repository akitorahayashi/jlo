//! Delete role under `.jlo/roles/<layer>/<name>/` and unschedule it.

use crate::app::AppContext;
use crate::domain::PromptAssetLoader;
use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{JloStore, JulesStore, RepositoryFilesystem, RoleTemplateStore};

use crate::app::commands::role_schedule::remove_role_scheduled;

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
        .ok_or_else(|| AppError::InvalidLayer { name: layer.to_string() })?;
    if layer_enum.is_single_role() {
        return Err(AppError::SingleRoleLayerTemplate(layer_enum.dir_name().to_string()));
    }

    let role_id = RoleId::new(role)?;
    let role_dir = format!(".jlo/roles/{}/{}", layer_enum.dir_name(), role_id.as_str());
    let role_yml = format!("{}/role.yml", role_dir);

    if !ctx.repository().file_exists(&role_yml) {
        return Err(AppError::RoleNotFound(format!(
            "{}/{}",
            layer_enum.dir_name(),
            role_id.as_str()
        )));
    }

    let removed = remove_role_scheduled(ctx.repository(), layer_enum, &role_id)?;
    if !removed {
        return Err(AppError::RoleNotInConfig {
            role: role_id.as_str().to_string(),
            layer: layer_enum.dir_name().to_string(),
        });
    }

    ctx.repository().remove_dir_all(&role_dir)?;

    Ok(RoleDeleteOutcome::Role {
        layer: layer_enum.dir_name().to_string(),
        role: role_id.as_str().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::catalogs::EmbeddedRoleTemplateStore;
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
jlo_target_branch = "main"
jules_worker_branch = "jules"

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
jlo_target_branch = "main"
jules_worker_branch = "jules"

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
        assert!(matches!(err, AppError::RoleNotInConfig { .. }));
    }
}
