//! Update command: advance `.jlo/` control-plane version pin and reconcile user
//! intent files.
//!
//! Update is a control-plane operation. It:
//! 1. Advances `.jlo/.jlo-version` to the current binary version.
//! 2. Creates any missing user intent files in `.jlo/` from scaffold defaults
//!    (config, schedules, role customizations, setup) without overwriting
//!    existing files.
//!
//! Update never reads or writes `.jules/` or runtime exchange artifacts.
//! Managed framework files (contracts, schemas, prompts) are materialized by
//! workflow bootstrap from the embedded scaffold for the pinned version.

use std::collections::BTreeMap;

use crate::app::commands::init;
use crate::domain::workspace::manifest::{
    MANIFEST_FILENAME, ScaffoldManifest, hash_content, is_control_plane_entity_file,
};
use crate::domain::{AppError, WorkflowRunnerMode};
use crate::ports::{RoleTemplateStore, WorkspaceStore};

/// Result of an update operation.
#[derive(Debug)]
pub struct UpdateResult {
    /// Files that were created (missing user intent files filled in).
    pub created: Vec<String>,
    /// Files that were updated (managed defaults refreshed).
    pub updated: Vec<String>,
    /// Whether workflow scaffold was refreshed.
    pub workflow_refreshed: bool,
    /// Whether this was a prompt preview.
    pub prompt_preview: bool,
    /// Previous version before the update (empty if same-version).
    pub previous_version: String,
    /// Non-fatal warnings encountered during update.
    pub warnings: Vec<String>,
}

/// Options for the update command.
#[derive(Debug, Default)]
pub struct UpdateOptions {
    /// Show planned changes without applying.
    pub prompt_preview: bool,
}

/// Execute the update command.
///
/// Operates exclusively on `.jlo/` control-plane files.
pub fn execute<W>(
    workspace: &W,
    options: UpdateOptions,
    templates: &impl RoleTemplateStore,
) -> Result<UpdateResult, AppError>
where
    W: WorkspaceStore,
{
    // Check if control plane exists
    if !workspace.jlo_exists() {
        return Err(AppError::Validation(
            "No .jlo/ control plane found. Run 'jlo init' first.".to_string(),
        ));
    }

    let version_path = ".jlo/.jlo-version";

    // Version comparison
    let binary_version = env!("CARGO_PKG_VERSION");
    let workspace_version = match workspace.read_file(version_path) {
        Ok(content) => content.trim().to_string(),
        Err(_) => {
            return Err(AppError::WorkspaceIntegrity(
                "Missing .jlo/.jlo-version file. Cannot update without version marker.".into(),
            ));
        }
    };

    // Helper to parse version string, ignoring pre-release suffixes (e.g. 1.2.3-beta -> 1.2.3)
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('-').next().unwrap_or(v).split('.').filter_map(|s| s.parse().ok()).collect()
    };

    let binary_parts = parse_version(binary_version);
    let workspace_parts = parse_version(&workspace_version);

    let version_cmp = compare_versions(&binary_parts, &workspace_parts);

    if version_cmp < 0 {
        return Err(AppError::WorkspaceVersionMismatch {
            workspace: workspace_version,
            binary: binary_version.into(),
        });
    }

    // Load control-plane skeleton files only — entity files (roles, schedules)
    // are not recreated during update to respect intentional deletions.
    let control_plane_files = templates.control_plane_skeleton_files();
    let mut to_create: Vec<(String, String)> = Vec::new();

    for file in &control_plane_files {
        // Skip the version pin — it is written explicitly below
        if file.path == ".jlo/.jlo-version" {
            continue;
        }
        // Only create missing files; never overwrite user-owned content
        if !workspace.file_exists(&file.path) {
            to_create.push((file.path.clone(), file.content.clone()));
        }
    }

    // Build entity default map for controlled refresh
    let mut entity_defaults = BTreeMap::new();
    for file in templates.control_plane_files() {
        if is_control_plane_entity_file(&file.path) {
            entity_defaults.insert(file.path.clone(), file.content.clone());
        }
    }

    let manifest_path = format!(".jlo/{}", MANIFEST_FILENAME);
    let mut warnings = Vec::new();
    let mut manifest_map: BTreeMap<String, String> = if workspace.file_exists(&manifest_path) {
        let content = workspace.read_file(&manifest_path)?;
        ScaffoldManifest::from_yaml(&content)?.to_map()
    } else {
        warnings.push(
            "Missing .jlo-managed.yml; defaults will only be recorded when files already match current embedded templates."
                .to_string(),
        );
        BTreeMap::new()
    };

    let mut to_update: Vec<(String, String)> = Vec::new();
    let mut manifest_changed = false;

    for (path, default_content) in &entity_defaults {
        if !workspace.file_exists(path) {
            if manifest_map.remove(path).is_some() {
                manifest_changed = true;
            }
            continue;
        }

        let current_content = workspace.read_file(path)?;
        let current_hash = hash_content(&current_content);
        let default_hash = hash_content(default_content);

        match manifest_map.get(path) {
            Some(stored_hash) if stored_hash == &current_hash => {
                if current_content != *default_content {
                    to_update.push((path.clone(), default_content.clone()));
                }
                if stored_hash != &default_hash {
                    manifest_map.insert(path.clone(), default_hash);
                    manifest_changed = true;
                }
            }
            Some(_) => {
                // User-customized; stop managing this file.
                manifest_map.remove(path);
                manifest_changed = true;
            }
            None => {
                // Record only when it already matches current defaults.
                if current_content == *default_content {
                    manifest_map.insert(path.clone(), default_hash);
                    manifest_changed = true;
                }
            }
        }
    }

    let workflow_mode = detect_workflow_mode(workspace)?;
    let workflow_will_refresh = workflow_mode.is_some();

    // Prompt preview
    if options.prompt_preview {
        println!("=== Prompt Preview: Update Plan ===\n");
        println!("Current version: {}", workspace_version);
        println!("Target version:  {}\n", binary_version);

        if to_create.is_empty() {
            println!("No new control-plane files to create.");
        } else {
            println!("Control-plane files to create:");
            for (path, _) in &to_create {
                println!("  • {}", path);
            }
        }

        if to_update.is_empty() {
            println!("No managed defaults to refresh.");
        } else {
            println!("Managed defaults to refresh:");
            for (path, _) in &to_update {
                println!("  • {}", path);
            }
        }

        if workflow_will_refresh {
            println!("Workflow scaffold will be refreshed.");
        }

        if version_cmp > 0 {
            println!("Version pin will be advanced.");
        } else {
            println!("Version pin will remain unchanged.");
        }

        return Ok(UpdateResult {
            created: to_create.into_iter().map(|(p, _)| p).collect(),
            updated: to_update.into_iter().map(|(p, _)| p).collect(),
            workflow_refreshed: workflow_will_refresh,
            prompt_preview: true,
            previous_version: workspace_version,
            warnings,
        });
    }

    // Create missing user intent files
    for (rel_path, content) in &to_create {
        workspace.write_file(rel_path, content)?;
    }

    // Refresh managed defaults
    for (rel_path, content) in &to_update {
        workspace.write_file(rel_path, content)?;
    }

    // Refresh workflow scaffold
    let mut workflow_refreshed = false;
    if let Some(mode) = workflow_mode {
        let root = workspace.resolve_path("");
        let generate_config = init::load_workflow_generate_config(&root)?;
        init::install_workflow_scaffold(&root, mode, &generate_config)?;
        workflow_refreshed = true;
    }

    if manifest_changed {
        let manifest = ScaffoldManifest::from_map(manifest_map);
        let manifest_content = manifest.to_yaml()?;
        workspace.write_file(&manifest_path, &manifest_content)?;
    }

    // Advance version pin if needed
    if version_cmp > 0 {
        workspace.write_file(version_path, &format!("{}\n", binary_version))?;
    }

    let created_paths: Vec<String> = to_create.into_iter().map(|(p, _)| p).collect();
    let updated_paths: Vec<String> = to_update.into_iter().map(|(p, _)| p).collect();

    Ok(UpdateResult {
        created: created_paths,
        updated: updated_paths,
        workflow_refreshed,
        prompt_preview: false,
        previous_version: workspace_version,
        warnings,
    })
}

fn detect_workflow_mode<W>(workspace: &W) -> Result<Option<WorkflowRunnerMode>, AppError>
where
    W: WorkspaceStore,
{
    let root = workspace.resolve_path("");
    match init::detect_workflow_runner_mode(&root) {
        Ok(mode) => Ok(Some(mode)),
        Err(_) => {
            // Workflow scaffold not found; skip refresh. This is normal for fresh workspaces
            // or in tests that don't set up a complete environment.
            Ok(None)
        }
    }
}

/// Compare two version arrays. Returns -1, 0, or 1.
fn compare_versions(a: &[u32], b: &[u32]) -> i32 {
    for i in 0..a.len().max(b.len()) {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        if av > bv {
            return 1;
        }
        if av < bv {
            return -1;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
    use std::fs;

    #[test]
    fn test_compare_versions() {
        assert_eq!(compare_versions(&[0, 1, 0], &[0, 1, 0]), 0);
        assert_eq!(compare_versions(&[0, 2, 0], &[0, 1, 0]), 1);
        assert_eq!(compare_versions(&[0, 1, 0], &[0, 2, 0]), -1);
        assert_eq!(compare_versions(&[1, 0, 0], &[0, 9, 9]), 1);
    }

    use crate::domain::Layer;
    use crate::ports::ScaffoldFile;
    use assert_fs::TempDir;

    fn sample_config_content() -> String {
        r#"[run]
default_branch = "main"
jules_branch = "jules"

[workflow]
cron = ["0 20 * * *"]
wait_minutes_default = 30
"#
        .to_string()
    }

    struct MockRoleTemplateStore {
        control_files: Vec<ScaffoldFile>,
    }

    impl RoleTemplateStore for MockRoleTemplateStore {
        fn scaffold_files(&self) -> Vec<ScaffoldFile> {
            vec![]
        }

        fn control_plane_files(&self) -> Vec<ScaffoldFile> {
            self.control_files.clone()
        }

        fn control_plane_skeleton_files(&self) -> Vec<ScaffoldFile> {
            self.control_files
                .iter()
                .filter(|f| !(f.path.ends_with("/role.yml") || f.path.ends_with("/scheduled.toml")))
                .cloned()
                .collect()
        }

        fn layer_template(&self, _layer: Layer) -> &str {
            ""
        }

        fn generate_role_yaml(&self, _role_id: &str, _layer: Layer) -> String {
            String::new()
        }
    }

    #[test]
    fn test_update_creates_missing_intent_files() {
        let temp = TempDir::new().unwrap();
        let jlo_path = temp.path().join(".jlo");
        fs::create_dir_all(&jlo_path).unwrap();

        let workflow_path = temp.path().join(".github/workflows");
        fs::create_dir_all(&workflow_path).unwrap();
        fs::write(
            workflow_path.join("jules-workflows.yml"),
            "jobs:\n  bootstrap:\n    runs-on: ubuntu-latest\n",
        )
        .unwrap();

        // Write version file (must be older than current to trigger update)
        fs::write(jlo_path.join(".jlo-version"), "0.0.0").unwrap();

        let mock_store = MockRoleTemplateStore {
            control_files: vec![
                ScaffoldFile {
                    path: ".jlo/config.toml".to_string(),
                    content: sample_config_content(),
                },
                ScaffoldFile {
                    path: ".jlo/setup/tools.yml".to_string(),
                    content: "tools: []".to_string(),
                },
            ],
        };

        let options = UpdateOptions { prompt_preview: false };
        let workspace = FilesystemWorkspaceStore::new(temp.path().to_path_buf());
        let result = execute(&workspace, options, &mock_store).unwrap();

        assert!(result.created.contains(&".jlo/config.toml".to_string()));
        assert!(result.created.contains(&".jlo/setup/tools.yml".to_string()));

        let config_path = temp.path().join(".jlo/config.toml");
        assert_eq!(fs::read_to_string(config_path).unwrap(), sample_config_content());
    }

    #[test]
    fn test_update_never_overwrites_existing_files() {
        let temp = TempDir::new().unwrap();
        let jlo_path = temp.path().join(".jlo");
        fs::create_dir_all(&jlo_path).unwrap();

        let workflow_path = temp.path().join(".github/workflows");
        fs::create_dir_all(&workflow_path).unwrap();
        fs::write(
            workflow_path.join("jules-workflows.yml"),
            "jobs:\n  bootstrap:\n    runs-on: ubuntu-latest\n",
        )
        .unwrap();

        fs::write(jlo_path.join(".jlo-version"), "0.0.0").unwrap();
        // User has customized config
        let custom_config = r#"[run]
    default_branch = "custom"
    jules_branch = "custom-jules"

    [workflow]
    cron = ["0 12 * * 1-5"]
    wait_minutes_default = 45
    "#;
        fs::write(jlo_path.join("config.toml"), custom_config).unwrap();

        let mock_store = MockRoleTemplateStore {
            control_files: vec![ScaffoldFile {
                path: ".jlo/config.toml".to_string(),
                content: sample_config_content(),
            }],
        };

        let options = UpdateOptions { prompt_preview: false };
        let workspace = FilesystemWorkspaceStore::new(temp.path().to_path_buf());
        let result = execute(&workspace, options, &mock_store).unwrap();

        // Should NOT overwrite user-owned file
        assert!(!result.created.contains(&".jlo/config.toml".to_string()));

        let config_path = temp.path().join(".jlo/config.toml");
        assert_eq!(fs::read_to_string(config_path).unwrap(), custom_config);
    }

    #[test]
    fn test_update_advances_version_pin() {
        let temp = TempDir::new().unwrap();
        let jlo_path = temp.path().join(".jlo");
        fs::create_dir_all(&jlo_path).unwrap();

        fs::write(jlo_path.join(".jlo-version"), "0.0.0").unwrap();

        let mock_store = MockRoleTemplateStore { control_files: vec![] };

        let options = UpdateOptions { prompt_preview: false };
        let workspace = FilesystemWorkspaceStore::new(temp.path().to_path_buf());
        let result = execute(&workspace, options, &mock_store).unwrap();

        assert_eq!(result.previous_version, "0.0.0");
        let version = fs::read_to_string(jlo_path.join(".jlo-version")).unwrap();
        assert_eq!(version.trim(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_update_succeeds_when_current_version() {
        let temp = TempDir::new().unwrap();
        let jlo_path = temp.path().join(".jlo");
        fs::create_dir_all(&jlo_path).unwrap();

        fs::write(jlo_path.join(".jlo-version"), env!("CARGO_PKG_VERSION")).unwrap();

        let mock_store = MockRoleTemplateStore { control_files: vec![] };

        let options = UpdateOptions { prompt_preview: false };
        let workspace = FilesystemWorkspaceStore::new(temp.path().to_path_buf());
        let result = execute(&workspace, options, &mock_store).unwrap();

        assert!(result.created.is_empty());
        assert_eq!(result.previous_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_update_does_not_recreate_deleted_entities() {
        let temp = TempDir::new().unwrap();
        let jlo_path = temp.path().join(".jlo");
        fs::create_dir_all(&jlo_path).unwrap();

        // Write version file (must be older to trigger update)
        fs::write(jlo_path.join(".jlo-version"), "0.0.0").unwrap();

        // Simulate a scaffold that ships both skeleton and entity files
        let mock_store = MockRoleTemplateStore {
            control_files: vec![
                ScaffoldFile {
                    path: ".jlo/config.toml".to_string(),
                    content: sample_config_content(),
                },
                ScaffoldFile {
                    path: ".jlo/roles/observers/default/role.yml".to_string(),
                    content: "role: default".to_string(),
                },
                ScaffoldFile {
                    path: ".jlo/workstreams/generic/scheduled.toml".to_string(),
                    content: "enabled = true".to_string(),
                },
            ],
        };

        let options = UpdateOptions { prompt_preview: false };
        let workspace = FilesystemWorkspaceStore::new(temp.path().to_path_buf());
        let result = execute(&workspace, options, &mock_store).unwrap();

        // Skeleton file (config.toml) should be created
        assert!(result.created.contains(&".jlo/config.toml".to_string()));

        // Entity files should NOT be created — user may have intentionally deleted them
        assert!(!result.created.contains(&".jlo/roles/observers/default/role.yml".to_string()));
        assert!(!result.created.contains(&".jlo/workstreams/generic/scheduled.toml".to_string()));

        // Verify files on disk
        assert!(temp.path().join(".jlo/config.toml").exists());
        assert!(!temp.path().join(".jlo/roles/observers/default/role.yml").exists());
        assert!(!temp.path().join(".jlo/workstreams/generic/scheduled.toml").exists());
    }
}
