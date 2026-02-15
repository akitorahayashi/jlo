use crate::adapters::control_plane_config;
use crate::adapters::workflow_installer;
use crate::app::AppContext;
use crate::domain::PromptAssetLoader;
use crate::domain::{AppError, WorkflowRunnerMode};
use crate::domain::{JLO_DIR, VERSION_FILE};
use crate::ports::{Git, JloStore, JulesStore, RepositoryFilesystem, RoleTemplateStore};

/// Execute the unified init command.
///
/// Creates the `.jlo/` control plane, the `.jules/` runtime repository, and
/// installs the workflow scaffold into `.github/`.
pub fn execute<W, R, G>(
    ctx: &AppContext<W, R>,
    git: &G,
    mode: &WorkflowRunnerMode,
) -> Result<(), AppError>
where
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
    R: RoleTemplateStore,
    G: Git,
{
    if ctx.repository().jlo_exists() {
        return Err(AppError::JloAlreadyExists);
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
        ctx.repository().write_file(&entry.path, &entry.content)?;
    }

    // Delegate config persistence
    control_plane_config::persist_workflow_runner_mode(ctx.repository(), mode)?;

    // Write version pin to .jlo/
    let jlo_version_path = format!("{}/{}", JLO_DIR, VERSION_FILE);
    ctx.repository().write_file(&jlo_version_path, &format!("{}\n", env!("CARGO_PKG_VERSION")))?;

    // Install workflow scaffold
    let generate_config = control_plane_config::load_workflow_generate_config(ctx.repository())?;
    workflow_installer::install_workflow_scaffold(ctx.repository(), mode, &generate_config)?;

    // Generate setup artifacts immediately in control plane.
    // Hard-fail init when setup generation fails.
    crate::app::commands::setup::generate(ctx.repository())?;

    Ok(())
}
