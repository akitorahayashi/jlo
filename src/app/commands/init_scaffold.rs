use std::collections::BTreeMap;

use crate::app::AppContext;
use crate::app::commands::init_workflows;
use crate::domain::workspace::manifest::{MANIFEST_FILENAME, hash_content, is_default_role_file};
use crate::domain::{AppError, ScaffoldManifest, WorkflowRunnerMode};
use crate::ports::{GitPort, RoleTemplateStore, WorkspaceStore};

/// Execute the unified init command.
///
/// Creates the `.jlo/` control plane, the `.jules/` runtime workspace, and
/// installs the workflow kit into `.github/`.
pub fn execute<W, R, G>(
    ctx: &AppContext<W, R>,
    git: &G,
    mode: WorkflowRunnerMode,
) -> Result<(), AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
    G: GitPort,
{
    if ctx.workspace().jlo_exists() {
        return Err(AppError::WorkspaceExists);
    }

    // Reject execution on 'jules' branch — init creates the control plane which
    // belongs on the user's control branch, not the runtime branch.
    let branch = git.get_current_branch()?;
    if branch == "jules" {
        return Err(AppError::Validation(
            "Init must not be run on the 'jules' branch. The 'jules' branch is the runtime branch managed by workflow bootstrap.\nRun init on your control branch (e.g. main, development).".to_string(),
        ));
    }

    // Create .jlo/ control plane (minimal intent overlay)
    let control_plane_files = ctx.templates().control_plane_files();
    for entry in &control_plane_files {
        ctx.workspace().write_file(&entry.path, &entry.content)?;
    }

    // Write version pin to .jlo/
    let jlo_version_path = ".jlo/.jlo-version";
    ctx.workspace().write_file(jlo_version_path, &format!("{}\n", env!("CARGO_PKG_VERSION")))?;

    // Create .jules/ runtime workspace (for local development convenience)
    let scaffold_files = ctx.templates().scaffold_files();
    ctx.workspace().create_structure(&scaffold_files)?;
    ctx.workspace().write_version(env!("CARGO_PKG_VERSION"))?;

    // Create managed manifest for .jules/
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

    // Install workflow kit
    let root = ctx.workspace().resolve_path("");
    init_workflows::execute_workflows(&root, mode)?;

    Ok(())
}
