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

use crate::domain::AppError;
use crate::ports::{RoleTemplateStore, WorkspaceStore};

/// Result of an update operation.
#[derive(Debug)]
pub struct UpdateResult {
    /// Files that were created (missing user intent files filled in).
    pub created: Vec<String>,
    /// Whether this was a prompt preview.
    pub prompt_preview: bool,
    /// Previous version before the update (empty if same-version).
    pub previous_version: String,
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

    let binary_parts: Vec<u32> = binary_version.split('.').filter_map(|s| s.parse().ok()).collect();
    let workspace_parts: Vec<u32> =
        workspace_version.split('.').filter_map(|s| s.parse().ok()).collect();

    let version_cmp = compare_versions(&binary_parts, &workspace_parts);

    if version_cmp < 0 {
        return Err(AppError::WorkspaceVersionMismatch {
            workspace: workspace_version,
            binary: binary_version.into(),
        });
    }

    if version_cmp == 0 {
        println!("Workspace is already at version {}. Nothing to update.", binary_version);
        return Ok(UpdateResult {
            created: vec![],
            prompt_preview: options.prompt_preview,
            previous_version: workspace_version,
        });
    }

    // Load control-plane files from scaffold and find missing user intent files
    let control_plane_files = templates.control_plane_files();
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

    // Prompt preview
    if options.prompt_preview {
        println!("=== Prompt Preview: Update Plan ===\n");
        println!("Current version: {}", workspace_version);
        println!("Target version:  {}\n", binary_version);

        if to_create.is_empty() {
            println!("Version pin will be advanced. No new files to create.");
        } else {
            println!("Files to create:");
            for (path, _) in &to_create {
                println!("  • {}", path);
            }
        }

        return Ok(UpdateResult {
            created: to_create.into_iter().map(|(p, _)| p).collect(),
            prompt_preview: true,
            previous_version: workspace_version,
        });
    }

    // Create missing user intent files
    for (rel_path, content) in &to_create {
        workspace.write_file(rel_path, content)?;
    }

    // Advance version pin
    workspace.write_file(version_path, &format!("{}\n", binary_version))?;

    let created_paths: Vec<String> = to_create.into_iter().map(|(p, _)| p).collect();

    Ok(UpdateResult {
        created: created_paths,
        prompt_preview: false,
        previous_version: workspace_version,
    })
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

        // Write version file (must be older than current to trigger update)
        fs::write(jlo_path.join(".jlo-version"), "0.0.0").unwrap();

        let mock_store = MockRoleTemplateStore {
            control_files: vec![
                ScaffoldFile {
                    path: ".jlo/config.toml".to_string(),
                    content: "# config".to_string(),
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
        assert_eq!(fs::read_to_string(config_path).unwrap(), "# config");
    }

    #[test]
    fn test_update_never_overwrites_existing_files() {
        let temp = TempDir::new().unwrap();
        let jlo_path = temp.path().join(".jlo");
        fs::create_dir_all(&jlo_path).unwrap();

        fs::write(jlo_path.join(".jlo-version"), "0.0.0").unwrap();
        // User has customized config
        fs::write(jlo_path.join("config.toml"), "user_custom = true").unwrap();

        let mock_store = MockRoleTemplateStore {
            control_files: vec![ScaffoldFile {
                path: ".jlo/config.toml".to_string(),
                content: "# default config".to_string(),
            }],
        };

        let options = UpdateOptions { prompt_preview: false };
        let workspace = FilesystemWorkspaceStore::new(temp.path().to_path_buf());
        let result = execute(&workspace, options, &mock_store).unwrap();

        // Should NOT overwrite user-owned file
        assert!(!result.created.contains(&".jlo/config.toml".to_string()));

        let config_path = temp.path().join(".jlo/config.toml");
        assert_eq!(fs::read_to_string(config_path).unwrap(), "user_custom = true");
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
    fn test_update_noop_when_current_version() {
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
}
