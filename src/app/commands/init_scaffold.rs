use std::collections::BTreeMap;

use crate::app::AppContext;
use crate::domain::workspace::manifest::{MANIFEST_FILENAME, hash_content, is_default_role_file};
use crate::domain::{AppError, ScaffoldManifest};
use crate::ports::{GitPort, RoleTemplateStore, WorkspaceStore};

/// Execute the init command.
///
/// Creates both the `.jules/` workspace and `.jules/setup/` directory.
pub fn execute<W, R, G>(ctx: &AppContext<W, R>, git: &G) -> Result<(), AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
    G: GitPort,
{
    if ctx.workspace().exists() {
        return Err(AppError::WorkspaceExists);
    }

    // Enforce execution on 'jules' or 'jules-test-*' branch to protect main history
    let branch = git.get_current_branch()?;

    if branch != "jules" && !branch.starts_with("jules-test-") {
        return Err(AppError::Validation(format!(
            "Init must be run on 'jules' or 'jules-test-*' branch (current: '{}').\nRun: git checkout -b jules (or git checkout -b jules-test-<name>)",
            branch
        )));
    }

    let scaffold_files = ctx.templates().scaffold_files();
    ctx.workspace().create_structure(&scaffold_files)?;

    ctx.workspace().write_version(env!("CARGO_PKG_VERSION"))?;

    // Create managed manifest
    let mut map = BTreeMap::new();
    for file in &scaffold_files {
        if is_default_role_file(&file.path) {
            map.insert(file.path.clone(), hash_content(&file.content));
        }
    }
    let managed_manifest = ScaffoldManifest::from_map(map);
    let manifest_content = managed_manifest.to_yaml()?;
    let manifest_path = format!(".jules/{}", MANIFEST_FILENAME);
    ctx.workspace().write_file(&manifest_path, &manifest_content)?;

    Ok(())
}
