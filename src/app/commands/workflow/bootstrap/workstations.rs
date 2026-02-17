//! Workflow bootstrap workstations subcommand.
//!
//! Reconciles workstation perspectives from schedule intent.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use serde::Serialize;

use crate::adapters::local_repository::LocalRepositoryAdapter;
use crate::domain::PromptAssetLoader;
use crate::domain::{AppError, Layer};
use crate::ports::{JloStore, JulesStore, RepositoryFilesystem};

/// Options for `workflow bootstrap workstations`.
#[derive(Debug)]
pub struct WorkflowBootstrapWorkstationsOptions {
    /// Root path of the repository.
    pub root: std::path::PathBuf,
}

/// Output of `workflow bootstrap workstations`.
#[derive(Debug, Serialize)]
pub struct WorkflowBootstrapWorkstationsOutput {
    /// Whether reconciliation was performed.
    pub reconciled: bool,
    /// Number of newly created perspective files.
    pub perspectives_created: usize,
    /// Number of removed stale workstation directories.
    pub workstations_pruned: usize,
}

#[derive(Debug)]
struct WorkstationReconcileStats {
    perspectives_created: usize,
    workstations_pruned: usize,
}

/// Execute `workflow bootstrap workstations`.
pub fn execute(
    options: WorkflowBootstrapWorkstationsOptions,
) -> Result<WorkflowBootstrapWorkstationsOutput, AppError> {
    super::validate_control_plane_preconditions(options.root.as_path())?;

    let repository = LocalRepositoryAdapter::new(options.root);
    let stats = reconcile_workstations(&repository)?;

    Ok(WorkflowBootstrapWorkstationsOutput {
        reconciled: true,
        perspectives_created: stats.perspectives_created,
        workstations_pruned: stats.workstations_pruned,
    })
}

fn reconcile_workstations<W>(repository: &W) -> Result<WorkstationReconcileStats, AppError>
where
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
{
    let schedule = crate::app::config::load_schedule(repository)?;
    let jules_path = repository.jules_path();
    let workstations_dir = crate::domain::workstations::paths::workstations_dir(&jules_path);
    let workstations_dir_str = path_to_str(&workstations_dir, "workstations path")?;

    let role_layers = collect_scheduled_role_layers(&schedule)?;
    let mut expected_roles: BTreeSet<String> = BTreeSet::new();
    let mut perspectives_created = 0usize;
    let mut workstations_pruned = 0usize;

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
        let rendered_perspective =
            materialize_perspective_from_schema(&schema_content, *layer, role)?;
        let workstation_dir =
            crate::domain::workstations::paths::workstation_dir(&jules_path, role);
        let workstation_dir_str = path_to_str(&workstation_dir, "workstation directory path")?;
        repository.create_dir_all(workstation_dir_str)?;
        repository.write_file(perspective_path_str, &rendered_perspective)?;
        perspectives_created += 1;
    }

    let entries = match repository.list_dir(workstations_dir_str) {
        Ok(entries) => entries,
        Err(AppError::Io { kind: crate::domain::IoErrorKind::NotFound, .. }) => {
            return Ok(WorkstationReconcileStats { perspectives_created, workstations_pruned });
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
        workstations_pruned += 1;
    }

    Ok(WorkstationReconcileStats { perspectives_created, workstations_pruned })
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

fn materialize_perspective_from_schema(
    schema_content: &str,
    layer: Layer,
    role: &str,
) -> Result<String, AppError> {
    let mut root: serde_yaml::Value = serde_yaml::from_str(schema_content).map_err(|err| {
        AppError::RepositoryIntegrity(format!(
            "Invalid perspective schema YAML for layer '{}': {}",
            layer.dir_name(),
            err
        ))
    })?;
    let map = root.as_mapping_mut().ok_or_else(|| {
        AppError::RepositoryIntegrity(format!(
            "Perspective schema root must be a mapping for layer '{}'",
            layer.dir_name()
        ))
    })?;

    let key = layer.perspective_role_key()?;
    map.insert(
        serde_yaml::Value::String(key.to_string()),
        serde_yaml::Value::String(role.to_string()),
    );

    serde_yaml::to_string(&root).map_err(|err| {
        AppError::RepositoryIntegrity(format!(
            "Failed to render perspective for layer '{}': {}",
            layer.dir_name(),
            err
        ))
    })
}
