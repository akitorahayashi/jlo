//! Upgrade command: advance `.jlo/` control-plane version pin and reconcile
//! skeleton files.
//!
//! Upgrade is a control-plane operation. It:
//! 1. Advances `.jlo/.jlo-version` to the current binary version.
//! 2. Creates any missing control-plane skeleton files in `.jlo/` from scaffold
//!    defaults without overwriting existing files.
//!
//! Upgrade never reads or writes `.jules/` or runtime exchange artifacts.
//! Managed framework files (contracts, schemas, prompts) are materialized by
//! workflow bootstrap from the embedded scaffold for the pinned version.

use crate::domain::{AppError, PromptAssetLoader, WorkflowRunnerMode};
use crate::ports::{JloStore, JulesStore, RepositoryFilesystem, RoleTemplateStore};

/// Result of an upgrade operation.
#[derive(Debug)]
pub struct UpgradeResult {
    /// Files that were created (missing skeleton files filled in).
    pub created: Vec<String>,
    /// Files that were updated (reserved for backward-compatible output shape).
    pub updated: Vec<String>,
    /// Whether workflow scaffold was refreshed.
    pub workflow_refreshed: bool,
    /// Whether this was a prompt preview.
    pub prompt_preview: bool,
    /// Previous version before the upgrade (empty if same-version).
    pub previous_version: String,
    /// Non-fatal warnings encountered during upgrade (currently unused).
    pub warnings: Vec<String>,
}

/// Options for the upgrade command.
#[derive(Debug, Default)]
pub struct UpgradeOptions {
    /// Show planned changes without applying.
    pub prompt_preview: bool,
}

/// Execute the upgrade command.
///
/// Operates exclusively on `.jlo/` control-plane files.
pub fn execute<W>(
    repository: &W,
    options: UpgradeOptions,
    templates: &impl RoleTemplateStore,
) -> Result<UpgradeResult, AppError>
where
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
{
    // Check if control plane exists
    if !repository.jlo_exists() {
        return Err(AppError::Validation(
            "No .jlo/ control plane found. Run 'jlo init' first.".to_string(),
        ));
    }

    let version_path = ".jlo/.jlo-version";

    // Version comparison
    let binary_version = env!("CARGO_PKG_VERSION");
    let runtime_version = match repository.read_file(version_path) {
        Ok(content) => content.trim().to_string(),
        Err(_) => {
            return Err(AppError::RepositoryIntegrity(
                "Missing .jlo/.jlo-version file. Cannot upgrade without version marker.".into(),
            ));
        }
    };

    // Helper to parse version string, ignoring pre-release suffixes (e.g. 1.2.3-beta -> 1.2.3)
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('-').next().unwrap_or(v).split('.').filter_map(|s| s.parse().ok()).collect()
    };

    let binary_parts = parse_version(binary_version);
    let runtime_parts = parse_version(&runtime_version);

    let version_cmp = compare_versions(&binary_parts, &runtime_parts);

    if version_cmp < 0 {
        return Err(AppError::RepositoryVersionMismatch {
            repository: runtime_version,
            binary: binary_version.into(),
        });
    }

    // Load control-plane skeleton files only.
    let control_plane_files = templates.control_plane_skeleton_files();
    let mut to_create: Vec<(String, String)> = Vec::new();

    for file in &control_plane_files {
        // Skip the version pin — it is written explicitly below
        if file.path == ".jlo/.jlo-version" {
            continue;
        }
        // Only create missing files; never overwrite user-owned content
        if !repository.file_exists(&file.path) {
            to_create.push((file.path.clone(), file.content.clone()));
        }
    }

    let to_update: Vec<(String, String)> = Vec::new();
    let warnings = Vec::new();

    let workflow_mode = configured_workflow_mode(repository)?;
    let workflow_will_refresh = workflow_mode.is_some();

    // Prompt preview
    if options.prompt_preview {
        println!("=== Prompt Preview: Upgrade Plan ===\n");
        println!("Current version: {}", runtime_version);
        println!("Target version:  {}\n", binary_version);

        if to_create.is_empty() {
            println!("No new control-plane files to create.");
        } else {
            println!("Control-plane files to create:");
            for (path, _) in &to_create {
                println!("  • {}", path);
            }
        }

        println!("No managed defaults to refresh.");

        if workflow_will_refresh {
            println!("Workflow scaffold will be refreshed.");
        }

        if version_cmp > 0 {
            println!("Version pin will be advanced.");
        } else {
            println!("Version pin will remain unchanged.");
        }

        return Ok(UpgradeResult {
            created: to_create.into_iter().map(|(p, _)| p).collect(),
            updated: to_update.into_iter().map(|(p, _)| p).collect(),
            workflow_refreshed: workflow_will_refresh,
            prompt_preview: true,
            previous_version: runtime_version,
            warnings,
        });
    }

    // Create missing skeleton files
    for (rel_path, content) in &to_create {
        repository.write_file(rel_path, content)?;
    }

    // Refresh workflow scaffold
    let mut workflow_refreshed = false;
    if let Some(mode) = workflow_mode {
        let generate_config =
            crate::adapters::control_plane_config::load_workflow_generate_config(repository)?;
        crate::adapters::workflow_installer::install_workflow_scaffold(
            repository,
            &mode,
            &generate_config,
        )?;
        workflow_refreshed = true;
    }

    // Advance version pin if needed
    if version_cmp > 0 {
        repository.write_file(version_path, &format!("{}\n", binary_version))?;
    }

    let created_paths: Vec<String> = to_create.into_iter().map(|(p, _)| p).collect();
    Ok(UpgradeResult {
        created: created_paths,
        updated: Vec::new(),
        workflow_refreshed,
        prompt_preview: false,
        previous_version: runtime_version,
        warnings,
    })
}

fn configured_workflow_mode<W>(repository: &W) -> Result<Option<WorkflowRunnerMode>, AppError>
where
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
{
    let has_managed_workflow = [
        ".github/workflows/jules-scheduled-workflows.yml",
        ".github/workflows/jules-workflows.yml",
        ".github/workflows/jules-sync.yml",
        ".github/workflows/jules-automerge.yml",
        ".github/workflows/jules-implementer-pr.yml",
        ".github/workflows/jules-integrator-pr.yml",
    ]
    .iter()
    .any(|path| repository.file_exists(path));

    if !has_managed_workflow {
        return Ok(None);
    }

    Ok(Some(crate::adapters::control_plane_config::load_workflow_runner_mode(repository)?))
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
    use crate::adapters::local_repository::LocalRepositoryAdapter;
    use std::fs;

    #[test]
    fn test_compare_versions() {
        assert_eq!(compare_versions(&[0, 1, 0], &[0, 1, 0]), 0);
        assert_eq!(compare_versions(&[0, 2, 0], &[0, 1, 0]), 1);
        assert_eq!(compare_versions(&[0, 1, 0], &[0, 2, 0]), -1);
        assert_eq!(compare_versions(&[1, 0, 0], &[0, 9, 9]), 1);
    }

    use crate::domain::{AppError, BuiltinRoleEntry, Layer};
    use crate::ports::ScaffoldFile;
    use assert_fs::TempDir;

    fn sample_config_content() -> String {
        r#"[run]
jlo_target_branch = "main"
jules_worker_branch = "jules"

[workflow]
runner_mode = "remote"
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
            self.control_files.iter().filter(|f| !f.path.ends_with("/role.yml")).cloned().collect()
        }

        fn layer_template(&self, _layer: Layer) -> &str {
            ""
        }

        fn generate_role_yaml(&self, _role_id: &str, _layer: Layer) -> String {
            String::new()
        }

        fn builtin_role_catalog(&self) -> Result<Vec<BuiltinRoleEntry>, AppError> {
            Ok(vec![])
        }

        fn builtin_role_content(&self, _layer: Layer, _role_id: &str) -> Result<String, AppError> {
            Err(AppError::Validation("builtin role content not available in mock".to_string()))
        }
    }

    #[test]
    fn test_upgrade_creates_missing_skeleton_files() {
        let temp = TempDir::new().unwrap();
        let jlo_path = temp.path().join(".jlo");
        fs::create_dir_all(&jlo_path).unwrap();

        // Write version file (must be older than current to trigger upgrade)
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

        let options = UpgradeOptions { prompt_preview: false };
        let repository = LocalRepositoryAdapter::new(temp.path().to_path_buf());
        let result = execute(&repository, options, &mock_store).unwrap();

        assert!(result.created.contains(&".jlo/config.toml".to_string()));
        assert!(result.created.contains(&".jlo/setup/tools.yml".to_string()));

        let config_path = temp.path().join(".jlo/config.toml");
        assert_eq!(fs::read_to_string(config_path).unwrap(), sample_config_content());
    }

    #[test]
    fn test_upgrade_never_overwrites_existing_files() {
        let temp = TempDir::new().unwrap();
        let jlo_path = temp.path().join(".jlo");
        fs::create_dir_all(&jlo_path).unwrap();

        fs::write(jlo_path.join(".jlo-version"), "0.0.0").unwrap();
        // User has customized config
        let custom_config = r#"[run]
    jlo_target_branch = "custom"
    jules_worker_branch = "custom-jules"

    [workflow]
    runner_mode = "remote"
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

        let options = UpgradeOptions { prompt_preview: false };
        let repository = LocalRepositoryAdapter::new(temp.path().to_path_buf());
        let result = execute(&repository, options, &mock_store).unwrap();

        // Should NOT overwrite user-owned file
        assert!(!result.created.contains(&".jlo/config.toml".to_string()));

        let config_path = temp.path().join(".jlo/config.toml");
        assert_eq!(fs::read_to_string(config_path).unwrap(), custom_config);
    }

    #[test]
    fn upgrade_fails_when_workflow_exists_but_runner_mode_missing() {
        let temp = TempDir::new().unwrap();
        let jlo_path = temp.path().join(".jlo");
        fs::create_dir_all(&jlo_path).unwrap();

        let workflow_path = temp.path().join(".github/workflows");
        fs::create_dir_all(&workflow_path).unwrap();
        fs::write(
            workflow_path.join("jules-scheduled-workflows.yml"),
            "name: Jules Scheduled Workflows\n",
        )
        .unwrap();

        fs::write(jlo_path.join(".jlo-version"), "0.0.0").unwrap();
        fs::write(
            jlo_path.join("config.toml"),
            r#"[run]
jlo_target_branch = "main"
jules_worker_branch = "jules"

[workflow]
cron = ["0 20 * * *"]
wait_minutes_default = 30
"#,
        )
        .unwrap();

        let mock_store = MockRoleTemplateStore { control_files: vec![] };
        let options = UpgradeOptions { prompt_preview: false };
        let repository = LocalRepositoryAdapter::new(temp.path().to_path_buf());
        let err = execute(&repository, options, &mock_store).unwrap_err();
        assert!(err.to_string().contains("workflow.runner_mode"));
    }

    #[test]
    fn test_upgrade_advances_version_pin() {
        let temp = TempDir::new().unwrap();
        let jlo_path = temp.path().join(".jlo");
        fs::create_dir_all(&jlo_path).unwrap();

        fs::write(jlo_path.join(".jlo-version"), "0.0.0").unwrap();

        let mock_store = MockRoleTemplateStore { control_files: vec![] };

        let options = UpgradeOptions { prompt_preview: false };
        let repository = LocalRepositoryAdapter::new(temp.path().to_path_buf());
        let result = execute(&repository, options, &mock_store).unwrap();

        assert_eq!(result.previous_version, "0.0.0");
        let version = fs::read_to_string(jlo_path.join(".jlo-version")).unwrap();
        assert_eq!(version.trim(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_upgrade_succeeds_when_current_version() {
        let temp = TempDir::new().unwrap();
        let jlo_path = temp.path().join(".jlo");
        fs::create_dir_all(&jlo_path).unwrap();

        fs::write(jlo_path.join(".jlo-version"), env!("CARGO_PKG_VERSION")).unwrap();

        let mock_store = MockRoleTemplateStore { control_files: vec![] };

        let options = UpgradeOptions { prompt_preview: false };
        let repository = LocalRepositoryAdapter::new(temp.path().to_path_buf());
        let result = execute(&repository, options, &mock_store).unwrap();

        assert!(result.created.is_empty());
        assert_eq!(result.previous_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_upgrade_does_not_recreate_deleted_entities() {
        let temp = TempDir::new().unwrap();
        let jlo_path = temp.path().join(".jlo");
        fs::create_dir_all(&jlo_path).unwrap();

        // Write version file (must be older to trigger upgrade)
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
            ],
        };

        let options = UpgradeOptions { prompt_preview: false };
        let repository = LocalRepositoryAdapter::new(temp.path().to_path_buf());
        let result = execute(&repository, options, &mock_store).unwrap();

        // Skeleton file (config.toml) should be created
        assert!(result.created.contains(&".jlo/config.toml".to_string()));

        // Entity files should NOT be created — user may have intentionally deleted them
        assert!(!result.created.contains(&".jlo/roles/observers/default/role.yml".to_string()));

        // Verify files on disk
        assert!(temp.path().join(".jlo/config.toml").exists());
        assert!(!temp.path().join(".jlo/roles/observers/default/role.yml").exists());
    }
}
