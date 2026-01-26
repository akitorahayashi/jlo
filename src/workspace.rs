//! Workspace operations for `.jules/` directory management.

use std::fs;
use std::path::PathBuf;

use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::scaffold;

/// The `.jules/` workspace directory name.
pub const JULES_DIR: &str = ".jules";

/// The version marker file.
pub const VERSION_FILE: &str = ".jo-version";

/// Represents a `.jules/` workspace rooted at a given path.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// The root directory containing `.jules/`.
    root: PathBuf,
}

impl Workspace {
    /// Create a workspace instance for the given root directory.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Create a workspace instance for the current directory.
    pub fn current() -> Result<Self, AppError> {
        let cwd = std::env::current_dir()?;
        Ok(Self::new(cwd))
    }

    /// Path to the `.jules/` directory.
    pub fn jules_path(&self) -> PathBuf {
        self.root.join(JULES_DIR)
    }

    /// Path to the `.jules/.jo-version` file.
    pub fn version_path(&self) -> PathBuf {
        self.jules_path().join(VERSION_FILE)
    }

    /// Check if a `.jules/` workspace exists.
    pub fn exists(&self) -> bool {
        self.jules_path().exists()
    }

    /// Read the current workspace version from `.jo-version`.
    pub fn read_version(&self) -> Result<Option<String>, AppError> {
        let path = self.version_path();
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        Ok(Some(content.trim().to_string()))
    }

    /// Write the version marker.
    pub fn write_version(&self, version: &str) -> Result<(), AppError> {
        fs::write(self.version_path(), format!("{}\n", version))?;
        Ok(())
    }

    /// Create the complete `.jules/` directory structure.
    pub fn create_structure(&self) -> Result<(), AppError> {
        fs::create_dir_all(self.jules_path())?;

        // Write scaffold files
        for entry in scaffold::scaffold_files() {
            let path = self.root.join(&entry.path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, entry.content)?;
        }

        // Scaffold all built-in roles so the structure is visible immediately.
        for role in scaffold::role_definitions() {
            if !self.role_exists(role.id) {
                self.scaffold_role(role.id, role.role_yaml, role.policy)?;
            }
        }

        Ok(())
    }

    /// Update jo-managed files and structural scaffolding.
    pub fn update_managed_files(&self) -> Result<(), AppError> {
        for entry in scaffold::update_managed_files() {
            let path = self.root.join(&entry.path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, entry.content)?;
        }
        Ok(())
    }

    /// List jo-managed files that are missing from the workspace.
    pub fn missing_managed_files(&self) -> Result<Vec<String>, AppError> {
        let mut missing = Vec::new();
        for entry in scaffold::update_managed_files() {
            let full_path = self.root.join(&entry.path);
            if !full_path.exists() {
                let display_path =
                    entry.path.strip_prefix(".jules/").unwrap_or(&entry.path).to_string();
                missing.push(display_path);
            }
        }
        missing.sort();
        Ok(missing)
    }

    /// Detect modified jo-managed files and structural placeholders by comparing content hashes.
    pub fn detect_modifications(&self) -> Result<Vec<String>, AppError> {
        let mut modified = Vec::new();
        for entry in scaffold::update_managed_files() {
            let full_path = self.root.join(&entry.path);
            if full_path.exists() {
                let actual_content = fs::read_to_string(&full_path)?;
                if hash_content(&actual_content) != hash_content(entry.content) {
                    let display_path =
                        entry.path.strip_prefix(".jules/").unwrap_or(&entry.path).to_string();
                    modified.push(display_path);
                }
            }
        }

        modified.sort();
        Ok(modified)
    }

    /// Path to a role directory.
    pub fn role_path(&self, role_id: &str) -> PathBuf {
        self.jules_path().join("roles").join(role_id)
    }

    /// Check if a role exists (has role.yml).
    pub fn role_exists(&self, role_id: &str) -> bool {
        if role_id.contains('/') || role_id.contains('\\') || role_id == "." || role_id == ".." {
            return false;
        }
        self.role_path(role_id).join("role.yml").exists()
    }

    /// Discover all existing roles by scanning for role.yml files.
    pub fn discover_roles(&self) -> Result<Vec<String>, AppError> {
        let roles_dir = self.jules_path().join("roles");
        if !roles_dir.exists() {
            return Ok(Vec::new());
        }

        let mut roles = Vec::new();
        for entry in fs::read_dir(&roles_dir)? {
            let entry = entry?;
            if !entry.path().is_dir() {
                continue;
            }
            let role_id = entry.file_name().to_string_lossy().to_string();
            if self.role_exists(&role_id) {
                roles.push(role_id);
            }
        }

        roles.sort();
        Ok(roles)
    }

    /// Scaffold a new role with role.yml and notes directory.
    pub fn scaffold_role(
        &self,
        role_id: &str,
        role_yaml: &str,
        policy: Option<&str>,
    ) -> Result<(), AppError> {
        let role_dir = self.role_path(role_id);
        let notes_dir = role_dir.join("notes");

        fs::create_dir_all(&notes_dir)?;
        fs::write(role_dir.join("role.yml"), role_yaml)?;
        fs::write(notes_dir.join(".gitkeep"), "")?;

        // Write policy file if provided (for PM role)
        if let Some(policy_content) = policy {
            fs::write(role_dir.join("policy.md"), policy_content)?;
        }

        Ok(())
    }

    /// Read the role configuration (role.yml).
    pub fn read_role_config(&self, role_id: &str) -> Result<String, AppError> {
        if !self.exists() {
            return Err(AppError::WorkspaceNotFound);
        }

        let config_path = self.role_path(role_id).join("role.yml");
        if !config_path.exists() {
            return Err(AppError::config_error(format!(
                "Role '{}' does not have role.yml",
                role_id
            )));
        }

        Ok(fs::read_to_string(config_path)?)
    }
}

/// Compute a SHA-256 hash of content for comparison.
fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_workspace() -> (TempDir, Workspace) {
        let dir = TempDir::new().expect("failed to create temp dir");
        let ws = Workspace::new(dir.path().to_path_buf());
        (dir, ws)
    }

    #[test]
    fn workspace_paths_are_correct() {
        let (_dir, ws) = test_workspace();
        assert!(ws.jules_path().ends_with(".jules"));
        assert!(ws.version_path().ends_with(".jo-version"));
    }

    #[test]
    fn create_structure_creates_directories() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().expect("create_structure should succeed");

        assert!(ws.jules_path().exists());
        assert!(ws.jules_path().join("roles").exists());
        assert!(ws.jules_path().join("README.md").exists());
        assert!(ws.jules_path().join("reports").exists());
        assert!(ws.jules_path().join("issues/bugs").exists());
        assert!(ws.jules_path().join("issues/refacts").exists());
        assert!(ws.jules_path().join("issues/updates").exists());
        assert!(ws.jules_path().join("issues/tests").exists());
        assert!(ws.jules_path().join("issues/docs").exists());
    }

    #[test]
    fn create_structure_scaffolds_all_builtin_roles() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().expect("create_structure should succeed");

        assert!(ws.role_exists("taxonomy"));
        assert!(ws.role_exists("data_arch"));
        assert!(ws.role_exists("qa"));
        assert!(ws.role_exists("pm"));
    }

    #[test]
    fn version_roundtrip() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().unwrap();

        ws.write_version("0.1.0").unwrap();
        let version = ws.read_version().unwrap();
        assert_eq!(version, Some("0.1.0".to_string()));
    }

    #[test]
    fn detect_modifications_empty_when_unchanged() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().unwrap();

        let mods = ws.detect_modifications().unwrap();
        assert!(mods.is_empty());
    }

    #[test]
    fn discover_roles_finds_existing() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().unwrap();
        ws.scaffold_role("test-role", "Test config", None).unwrap();

        let roles = ws.discover_roles().unwrap();
        assert!(roles.contains(&"test-role".to_string()));
    }

    #[test]
    fn scaffold_role_creates_structure() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().unwrap();

        ws.scaffold_role("my-role", "My role config", None).unwrap();

        let role_dir = ws.role_path("my-role");
        assert!(role_dir.join("role.yml").exists());
        assert!(role_dir.join("notes").exists());
        assert!(role_dir.join("notes/.gitkeep").exists());

        let config = fs::read_to_string(role_dir.join("role.yml")).unwrap();
        assert_eq!(config, "My role config");
    }

    #[test]
    fn scaffold_role_with_policy() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().unwrap();

        ws.scaffold_role("pm-test", "PM config", Some("PM policy content")).unwrap();

        let role_dir = ws.role_path("pm-test");
        assert!(role_dir.join("role.yml").exists());
        assert!(role_dir.join("policy.md").exists());

        let policy = fs::read_to_string(role_dir.join("policy.md")).unwrap();
        assert_eq!(policy, "PM policy content");
    }

    #[test]
    fn read_role_config_reads_file() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().unwrap();

        ws.scaffold_role("test", "Role specific instructions", None).unwrap();

        let config = ws.read_role_config("test").unwrap();
        assert!(config.contains("Role specific instructions"));
    }
}
