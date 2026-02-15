use std::collections::{BTreeMap, HashMap};

use crate::adapters::control_plane_config;
use crate::adapters::workflow_installer;
use crate::app::AppContext;
use crate::domain::workspace::manifest::{hash_content, is_control_plane_entity_file};
use crate::domain::workspace::paths::jlo;
use crate::domain::workspace::{JLO_DIR, VERSION_FILE};
use crate::domain::{AppError, Layer, ScaffoldManifest, Schedule, WorkflowRunnerMode};
use crate::ports::ScaffoldFile;
use crate::ports::{GitPort, RoleTemplateStore, WorkspaceStore};

/// Execute the unified init command.
///
/// Creates the `.jlo/` control plane, the `.jules/` runtime workspace, and
/// installs the workflow scaffold into `.github/`.
pub fn execute<W, R, G>(
    ctx: &AppContext<W, R>,
    git: &G,
    mode: &WorkflowRunnerMode,
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

    // Delegate config persistence
    control_plane_config::persist_workflow_runner_mode(ctx.workspace(), mode)?;

    let seeded_roles = seed_scheduled_roles(ctx)?;

    // Write version pin to .jlo/
    let jlo_version_path = format!("{}/{}", JLO_DIR, VERSION_FILE);
    ctx.workspace().write_file(&jlo_version_path, &format!("{}\n", env!("CARGO_PKG_VERSION")))?;

    // Create managed manifest for .jlo/ default entity files
    let mut map = BTreeMap::new();
    for file in control_plane_files.iter().chain(seeded_roles.iter()) {
        if is_control_plane_entity_file(&file.path) {
            map.insert(file.path.clone(), hash_content(&file.content));
        }
    }
    let managed_manifest = ScaffoldManifest::from_map(map);
    let manifest_content = managed_manifest.to_yaml()?;
    let manifest_path = jlo::manifest_relative();
    ctx.workspace().write_file(&manifest_path, &manifest_content)?;

    // Install workflow scaffold
    let generate_config = control_plane_config::load_workflow_generate_config(ctx.workspace())?;
    workflow_installer::install_workflow_scaffold(ctx.workspace(), mode, &generate_config)?;

    // Generate setup artifacts immediately in control plane.
    // Hard-fail init when setup generation fails.
    crate::app::commands::setup::generate(ctx.workspace())?;

    Ok(())
}

fn seed_scheduled_roles<W, R>(ctx: &AppContext<W, R>) -> Result<Vec<ScaffoldFile>, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
{
    let schedule_content = ctx.workspace().read_file(".jlo/scheduled.toml")?;
    let schedule = Schedule::parse_toml(&schedule_content)?;

    let catalog = ctx.templates().builtin_role_catalog()?;
    let mut catalog_index: HashMap<(String, String), ScaffoldFile> = HashMap::new();

    for entry in catalog {
        let content = ctx.templates().builtin_role_content(&entry.path)?;
        let path =
            format!(".jlo/roles/{}/{}/role.yml", entry.layer.dir_name(), entry.name.as_str());
        catalog_index.insert(
            (entry.layer.dir_name().to_string(), entry.name.as_str().to_string()),
            ScaffoldFile { path, content },
        );
    }

    let mut seeded = Vec::new();

    for role in &schedule.observers.roles {
        let key = (Layer::Observers.dir_name().to_string(), role.name.as_str().to_string());
        let file = catalog_index.get(&key).ok_or_else(|| {
            AppError::Validation(format!(
                "Scheduled observer role '{}' is missing from builtin catalog",
                role.name.as_str()
            ))
        })?;
        ctx.workspace().write_file(&file.path, &file.content)?;
        seeded.push(file.clone());
    }

    if let Some(ref innovators) = schedule.innovators {
        for role in &innovators.roles {
            let key = (Layer::Innovators.dir_name().to_string(), role.name.as_str().to_string());
            let file = catalog_index.get(&key).ok_or_else(|| {
                AppError::Validation(format!(
                    "Scheduled innovator role '{}' is missing from builtin catalog",
                    role.name.as_str()
                ))
            })?;
            ctx.workspace().write_file(&file.path, &file.content)?;
            seeded.push(file.clone());
        }
    }

    Ok(seeded)
}
