//! Update command implementation for reconciling workspace with embedded scaffold.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::domain::AppError;
use crate::ports::RoleTemplateStore;
use crate::services::EmbeddedRoleTemplateStore;
use crate::services::{
    ManagedDefaultsManifest, hash_content, is_default_role_file, load_manifest, write_manifest,
};

/// Files that are managed by jlo and will be overwritten on update.
///
/// Managed files are core framework files that:
/// - Define shared contracts and workflows (contracts.yml)
/// - Provide shared templates for agent outputs (event.yml, issue.yml, feedback.yml)
/// - Document system-wide rules and conventions (README.md, JULES.md)
///
/// Files NOT managed (user-customizable):
/// - Role-specific prompts (prompt.yml, role.yml)
/// - User configuration (config.toml)
/// - Workstream content (events/, issues/)
const JLO_MANAGED_FILES: &[&str] = &[
    ".jules/README.md",
    ".jules/JULES.md",
    ".jules/roles/observers/contracts.yml",
    ".jules/roles/observers/event.yml",
    ".jules/roles/deciders/contracts.yml",
    ".jules/roles/deciders/issue.yml",
    ".jules/roles/deciders/feedback.yml",
    ".jules/roles/planners/contracts.yml",
    ".jules/roles/implementers/contracts.yml",
];

/// Result of an update operation.
#[derive(Debug)]
pub struct UpdateResult {
    /// Files that were updated.
    pub updated: Vec<String>,
    /// Files that were created.
    pub created: Vec<String>,
    /// Files that were removed.
    pub removed: Vec<String>,
    /// Default role files skipped due to local changes or missing baseline.
    pub skipped: Vec<SkippedUpdate>,
    /// Whether this was a dry run.
    pub dry_run: bool,
    /// Backup directory path (if changes were made).
    pub backup_path: Option<PathBuf>,
    /// Whether a managed defaults baseline was adopted.
    pub adopted_managed: bool,
}

/// Options for the update command.
#[derive(Debug, Default)]
pub struct UpdateOptions {
    /// Show planned changes without applying.
    pub dry_run: bool,
    /// Include workflow files in update.
    /// TODO: Implement workflow file filtering in execute(). Currently unused.
    pub workflows: bool,
    /// Adopt current default role files as managed baseline (no conditional updates applied).
    pub adopt_managed: bool,
}

#[derive(Debug, Clone)]
pub struct SkippedUpdate {
    pub path: String,
    pub reason: String,
}

/// Execute the update command.
pub fn execute(jules_path: &Path, options: UpdateOptions) -> Result<UpdateResult, AppError> {
    let version_path = jules_path.join(".jlo-version");
    let root = jules_path.parent().unwrap_or(Path::new("."));

    // Check if workspace exists
    if !jules_path.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // Version comparison
    let binary_version = env!("CARGO_PKG_VERSION");
    let workspace_version = if version_path.exists() {
        fs::read_to_string(&version_path)?.trim().to_string()
    } else {
        return Err(AppError::ConfigError(
            "Missing .jlo-version file. Cannot update workspace without version marker.".into(),
        ));
    };

    // Parse versions for comparison
    let binary_parts: Vec<u32> = binary_version.split('.').filter_map(|s| s.parse().ok()).collect();
    let workspace_parts: Vec<u32> =
        workspace_version.split('.').filter_map(|s| s.parse().ok()).collect();

    // Compare versions
    let version_cmp = compare_versions(&binary_parts, &workspace_parts);

    if version_cmp < 0 {
        return Err(AppError::ConfigError(format!(
            "Workspace version ({}) is newer than binary version ({}). Update the jlo binary.",
            workspace_version, binary_version
        )));
    }

    if version_cmp == 0 && !options.adopt_managed {
        println!("Workspace is already at version {}. Nothing to update.", binary_version);
        return Ok(UpdateResult {
            updated: vec![],
            created: vec![],
            removed: vec![],
            skipped: vec![],
            dry_run: options.dry_run,
            backup_path: None,
            adopted_managed: false,
        });
    }

    // Load scaffold files
    let templates = EmbeddedRoleTemplateStore::new();
    let scaffold_files = templates.scaffold_files();

    // Plan updates
    let mut to_update: Vec<(String, String)> = Vec::new();
    let mut to_create: Vec<(String, String)> = Vec::new();
    let mut to_remove: Vec<String> = Vec::new();
    let mut skipped: Vec<SkippedUpdate> = Vec::new();
    let mut default_role_files: BTreeMap<String, String> = BTreeMap::new();

    for file in &scaffold_files {
        let rel_path = &file.path;

        // Check if this is a jlo-managed file
        if !is_jlo_managed(rel_path) {
            if is_default_role_file(rel_path) {
                default_role_files.insert(rel_path.clone(), file.content.clone());
                continue;
            }

            // For non-managed files, only create if missing
            let full_path = root.join(rel_path);
            if !full_path.exists() {
                to_create.push((rel_path.clone(), file.content.clone()));
            }
            continue;
        }

        // For jlo-managed files, always update
        let full_path = root.join(rel_path);
        if full_path.exists() {
            let current_content = fs::read_to_string(&full_path)?;
            if current_content != file.content {
                to_update.push((rel_path.clone(), file.content.clone()));
            }
        } else {
            to_create.push((rel_path.clone(), file.content.clone()));
        }
    }

    let mut managed_manifest: Option<ManagedDefaultsManifest> = None;
    let existing_manifest = load_manifest(jules_path)?;

    if options.adopt_managed {
        let mut manifest_entries = BTreeMap::new();
        for (path, content) in &default_role_files {
            let full_path = root.join(path);
            if full_path.exists() {
                let current = fs::read_to_string(&full_path)?;
                manifest_entries.insert(path.clone(), hash_content(&current));
            } else {
                to_create.push((path.clone(), content.clone()));
                manifest_entries.insert(path.clone(), hash_content(content));
            }
        }
        managed_manifest = Some(ManagedDefaultsManifest::from_map(manifest_entries));
    } else if let Some(manifest) = existing_manifest {
        let mut manifest_map = manifest.to_map();
        let mut next_manifest = BTreeMap::new();

        for (path, content) in &default_role_files {
            let full_path = root.join(path);
            if let Some(recorded_hash) = manifest_map.remove(path) {
                if full_path.exists() {
                    let current = fs::read_to_string(&full_path)?;
                    let current_hash = hash_content(&current);
                    if current_hash == recorded_hash {
                        if current != *content {
                            to_update.push((path.clone(), content.clone()));
                        }
                        next_manifest.insert(path.clone(), hash_content(content));
                    } else {
                        skipped.push(SkippedUpdate {
                            path: path.clone(),
                            reason: "Local changes detected; left untouched.".to_string(),
                        });
                        next_manifest.insert(path.clone(), recorded_hash);
                    }
                } else {
                    skipped.push(SkippedUpdate {
                        path: path.clone(),
                        reason: "File missing; treated as local removal and no longer tracked."
                            .to_string(),
                    });
                }
            } else if full_path.exists() {
                let current = fs::read_to_string(&full_path)?;
                if current == *content {
                    next_manifest.insert(path.clone(), hash_content(content));
                } else {
                    skipped.push(SkippedUpdate {
                        path: path.clone(),
                        reason: "Untracked file differs from default; not adopting.".to_string(),
                    });
                }
            } else {
                to_create.push((path.clone(), content.clone()));
                next_manifest.insert(path.clone(), hash_content(content));
            }
        }

        for (path, recorded_hash) in manifest_map {
            let full_path = root.join(&path);
            if full_path.exists() {
                let current = fs::read_to_string(&full_path)?;
                let current_hash = hash_content(&current);
                if current_hash == recorded_hash {
                    to_remove.push(path.clone());
                } else {
                    skipped.push(SkippedUpdate {
                        path: path.clone(),
                        reason:
                            "Default role removed upstream but modified locally; left in place."
                                .to_string(),
                    });
                }
            }
        }

        managed_manifest = Some(ManagedDefaultsManifest::from_map(next_manifest));
    } else {
        for (path, content) in &default_role_files {
            let full_path = root.join(path);
            if full_path.exists() {
                skipped.push(SkippedUpdate {
                    path: path.clone(),
                    reason: "Managed baseline missing; run update with --adopt-managed to track."
                        .to_string(),
                });
            } else {
                to_create.push((path.clone(), content.clone()));
            }
        }
    }

    // Dry run: just report planned changes
    if options.dry_run {
        println!("=== Dry Run: Update Plan ===\n");
        println!("Current version: {}", workspace_version);
        println!("Target version:  {}\n", binary_version);

        if to_update.is_empty() && to_create.is_empty() && to_remove.is_empty() {
            println!("No changes needed.");
        } else {
            if !to_update.is_empty() {
                println!("Files to update:");
                for (path, _) in &to_update {
                    println!("  • {}", path);
                }
            }
            if !to_create.is_empty() {
                println!("\nFiles to create:");
                for (path, _) in &to_create {
                    println!("  • {}", path);
                }
            }
            if !to_remove.is_empty() {
                println!("\nFiles to remove:");
                for path in &to_remove {
                    println!("  • {}", path);
                }
            }
            if !skipped.is_empty() {
                println!("\nFiles skipped:");
                for entry in &skipped {
                    println!("  • {} ({})", entry.path, entry.reason);
                }
            }
        }

        return Ok(UpdateResult {
            updated: to_update.into_iter().map(|(p, _)| p).collect(),
            created: to_create.into_iter().map(|(p, _)| p).collect(),
            removed: to_remove,
            skipped,
            dry_run: true,
            backup_path: None,
            adopted_managed: options.adopt_managed,
        });
    }

    // Create backup directory
    let backup_path = if !to_update.is_empty() || !to_remove.is_empty() {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_dir = jules_path.join(".jlo-update").join(&timestamp);
        fs::create_dir_all(&backup_dir)?;

        // Backup files that will be updated
        for (rel_path, _) in &to_update {
            let src = root.join(rel_path);
            let dst = backup_dir.join(rel_path);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src, &dst)?;
        }
        for rel_path in &to_remove {
            let src = root.join(rel_path);
            if src.exists() {
                let dst = backup_dir.join(rel_path);
                if let Some(parent) = dst.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&src, &dst)?;
            }
        }

        Some(backup_dir)
    } else {
        None
    };

    // Apply updates
    for (rel_path, content) in &to_update {
        let full_path = root.join(rel_path);
        fs::write(&full_path, content)?;
    }

    // Create new files
    for (rel_path, content) in &to_create {
        let full_path = root.join(rel_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&full_path, content)?;
    }

    for rel_path in &to_remove {
        let full_path = root.join(rel_path);
        if full_path.exists() {
            fs::remove_file(&full_path)?;
        }
    }

    if let Some(manifest) = managed_manifest {
        write_manifest(jules_path, &manifest)?;
    }

    // Update version file
    let mut version_file = fs::File::create(&version_path)?;
    writeln!(version_file, "{}", binary_version)?;

    let updated_paths: Vec<String> = to_update.into_iter().map(|(p, _)| p).collect();
    let created_paths: Vec<String> = to_create.into_iter().map(|(p, _)| p).collect();

    Ok(UpdateResult {
        updated: updated_paths,
        created: created_paths,
        removed: to_remove,
        skipped,
        dry_run: false,
        backup_path,
        adopted_managed: options.adopt_managed,
    })
}

/// Check if a file path is jlo-managed.
fn is_jlo_managed(path: &str) -> bool {
    JLO_MANAGED_FILES.contains(&path)
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

    #[test]
    fn test_compare_versions() {
        assert_eq!(compare_versions(&[0, 1, 0], &[0, 1, 0]), 0);
        assert_eq!(compare_versions(&[0, 2, 0], &[0, 1, 0]), 1);
        assert_eq!(compare_versions(&[0, 1, 0], &[0, 2, 0]), -1);
        assert_eq!(compare_versions(&[1, 0, 0], &[0, 9, 9]), 1);
    }

    #[test]
    fn test_is_jlo_managed() {
        assert!(is_jlo_managed(".jules/README.md"));
        assert!(is_jlo_managed(".jules/JULES.md"));
        assert!(is_jlo_managed(".jules/roles/observers/contracts.yml"));
        assert!(!is_jlo_managed(".jules/config.toml"));
        assert!(!is_jlo_managed(".jules/roles/observers/taxonomy/prompt.yml"));
    }
}
