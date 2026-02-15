//! Workflow bootstrap: deterministic projection of `.jules/` from `.jlo/` + scaffold.
//!
//! Runs as the first step in workflow execution, guaranteeing that the runtime
//! repository matches the control-plane intent before any agent job.
//!
//! Invariants:
//! - Missing `.jlo/` is a hard failure.
//! - Missing `.jlo/.jlo-version` is a hard failure.
//! - Managed framework files are always materialized from the embedded scaffold.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use serde::Serialize;

use crate::adapters::catalogs::EmbeddedRoleTemplateStore;
use crate::adapters::local_repository::LocalRepositoryAdapter;
use crate::app::AppContext;
use crate::domain::PromptAssetLoader;
use crate::domain::workstations::manifest::{
    MANIFEST_FILENAME, hash_content, is_default_role_file,
};
use crate::domain::{AppError, JLO_DIR, JULES_DIR, Layer, ScaffoldManifest, VERSION_FILE};
use crate::ports::{JloStore, JulesStore, RepositoryFilesystem, RoleTemplateStore};

/// Options for the bootstrap command.
#[derive(Debug)]
pub struct WorkflowBootstrapOptions {
    /// Root path of the repository (on the `jules` branch).
    pub root: std::path::PathBuf,
}

/// Output of the bootstrap command.
#[derive(Debug, Serialize)]
pub struct WorkflowBootstrapOutput {
    /// Whether materialization was performed.
    pub materialized: bool,
    /// The jlo version used for materialization.
    pub version: String,
    /// Number of files written during materialization.
    pub files_written: usize,
}

/// Execute the workflow bootstrap.
///
/// Deterministically projects `.jules/` from `.jlo/` control inputs and embedded scaffold.
pub fn execute(options: WorkflowBootstrapOptions) -> Result<WorkflowBootstrapOutput, AppError> {
    let current_version = env!("CARGO_PKG_VERSION");
    let root = &options.root;

    let store = LocalRepositoryAdapter::new(root.clone());
    let templates = EmbeddedRoleTemplateStore::new();

    // --- Hard preconditions ---
    let jlo_path = root.join(JLO_DIR);
    if !jlo_path.exists() {
        return Err(AppError::Validation(
            "Bootstrap requires .jlo/ control plane. Run 'jlo init' on your control branch first."
                .to_string(),
        ));
    }

    let jlo_version_path = jlo_path.join(VERSION_FILE);
    if !jlo_version_path.exists() {
        return Err(AppError::RepositoryIntegrity(
            "Missing .jlo/.jlo-version. Control plane is incomplete.".to_string(),
        ));
    }

    let ctx = AppContext::new(store, templates);
    let files_written = project_runtime(&ctx, root, current_version)?;

    Ok(WorkflowBootstrapOutput {
        materialized: true,
        version: current_version.to_string(),
        files_written,
    })
}

/// Full deterministic projection of `.jules/` from embedded scaffold.
fn project_runtime<W, R>(
    ctx: &AppContext<W, R>,
    _root: &Path,
    version: &str,
) -> Result<usize, AppError>
where
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
    R: RoleTemplateStore,
{
    // Counts write operations performed.
    let mut files_written: usize = 0;

    // 1. Materialize managed framework files from embedded scaffold
    let scaffold_files = ctx.templates().scaffold_files();
    ctx.repository().create_structure(&scaffold_files)?;
    ctx.repository().jules_write_version(version)?;
    files_written += scaffold_files.len() + 1; // +1 for version

    // 2. Ensure + prune workstation perspectives from schedule intent
    files_written += reconcile_workstations(ctx.repository())?;

    // 3. Write managed manifest
    let mut map = BTreeMap::new();
    for file in &scaffold_files {
        if is_default_role_file(&file.path) {
            map.insert(file.path.clone(), hash_content(&file.content));
        }
    }
    let managed_manifest = ScaffoldManifest::from_map(map);
    let manifest_content = managed_manifest.to_yaml()?;
    let manifest_path = format!("{}/{}", JULES_DIR, MANIFEST_FILENAME);
    ctx.repository().write_file(&manifest_path, &manifest_content)?;
    files_written += 1;

    Ok(files_written)
}

fn reconcile_workstations<W>(repository: &W) -> Result<usize, AppError>
where
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
{
    let schedule = crate::app::config::load_schedule(repository)?;
    let jules_path = repository.jules_path();
    let workstations_dir = crate::domain::workstations::paths::workstations_dir(&jules_path);
    let workstations_dir_str = path_to_str(&workstations_dir, "workstations path")?;

    let role_layers = collect_scheduled_role_layers(&schedule)?;
    let mut expected_roles: BTreeSet<String> = BTreeSet::new();
    let mut files_written = 0usize;

    for (role, layer) in &role_layers {
        expected_roles.insert(role.clone());

        let perspective_path =
            crate::domain::workstations::paths::workstation_perspective(&jules_path, role);
        let perspective_path_str = path_to_str(&perspective_path, "workstation perspective path")?;
        if repository.file_exists(perspective_path_str) {
            continue;
        }

        let schema_path =
            crate::domain::layers::paths::schemas_dir(&jules_path, *layer).join("perspective.yml");
        let schema_path_str = path_to_str(&schema_path, "perspective schema path")?;
        if !repository.file_exists(schema_path_str) {
            return Err(AppError::RepositoryIntegrity(format!(
                "Missing perspective schema for layer '{}': {}",
                layer.dir_name(),
                schema_path.display()
            )));
        }

        let schema_content = repository.read_file(schema_path_str)?;
        let rendered_perspective = match layer {
            Layer::Innovators => schema_content.replace("<persona_id>", role),
            Layer::Observers => schema_content.replace("<observer-id>", role),
            _ => schema_content,
        };
        let workstation_dir =
            crate::domain::workstations::paths::workstation_dir(&jules_path, role);
        let workstation_dir_str = path_to_str(&workstation_dir, "workstation directory path")?;
        repository.create_dir_all(workstation_dir_str)?;
        repository.write_file(perspective_path_str, &rendered_perspective)?;
        files_written += 1;
    }

    let entries = match repository.list_dir(workstations_dir_str) {
        Ok(entries) => entries,
        Err(AppError::Io { kind: crate::domain::IoErrorKind::NotFound, .. }) => {
            return Ok(files_written);
        }
        Err(err) => return Err(err),
    };

    for entry in entries {
        let Some(entry_str) = entry.to_str() else { continue };
        if !repository.is_dir(entry_str) {
            continue;
        }
        let Some(role_name) = entry.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if expected_roles.contains(role_name) {
            continue;
        }
        repository.remove_dir_all(entry_str)?;
    }

    Ok(files_written)
}

fn collect_scheduled_role_layers(
    schedule: &crate::domain::Schedule,
) -> Result<HashMap<String, Layer>, AppError> {
    let mut map: HashMap<String, Layer> = HashMap::new();

    for role in &schedule.observers.roles {
        let name = role.name.as_str().to_string();
        if let Some(previous) = map.insert(name.clone(), Layer::Observers) {
            return Err(AppError::Validation(format!(
                "Role '{}' is scheduled in both '{}' and '{}'; workstation ownership must be unique",
                name,
                previous.dir_name(),
                Layer::Observers.dir_name()
            )));
        }
    }

    if let Some(innovators) = &schedule.innovators {
        for role in &innovators.roles {
            let name = role.name.as_str().to_string();
            if let Some(previous) = map.insert(name.clone(), Layer::Innovators) {
                return Err(AppError::Validation(format!(
                    "Role '{}' is scheduled in both '{}' and '{}'; workstation ownership must be unique",
                    name,
                    previous.dir_name(),
                    Layer::Innovators.dir_name()
                )));
            }
        }
    }

    Ok(map)
}

fn path_to_str<'a>(path: &'a Path, label: &str) -> Result<&'a str, AppError> {
    path.to_str().ok_or_else(|| {
        AppError::InvalidPath(format!("Invalid unicode in {}: {}", label, path.display()))
    })
}
