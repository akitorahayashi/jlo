//! Workspace operations for `.jules/` directory management.

use crate::bundle::{self, jo_managed_map};
use crate::error::AppError;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

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
        let jules = self.jules_path();

        // Create all directories
        let dirs = [
            jules.clone(),
            jules.join("org"),
            jules.join("decisions"),
            jules.join("roles"),
            jules.join("exchange/inbox"),
            jules.join("exchange/threads"),
            jules.join("synthesis/weekly"),
            jules.join("state"),
            jules.join(".jo/policy"),
            jules.join(".jo/templates"),
        ];

        for dir in &dirs {
            fs::create_dir_all(dir)?;
        }

        // Write START_HERE.md
        fs::write(jules.join("START_HERE.md"), bundle::start_here_content())?;

        // Write org files
        for entry in bundle::org_files() {
            fs::write(jules.join(entry.path), entry.content)?;
        }

        // Write jo-managed files
        for entry in bundle::jo_managed_files() {
            let path = jules.join(entry.path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, entry.content)?;
        }

        // Write initial state files
        fs::write(jules.join("state/lenses.json"), "{}\n")?;
        fs::write(jules.join("state/open_threads.json"), "{}\n")?;

        Ok(())
    }

    /// Update jo-managed files under `.jules/.jo/`.
    pub fn update_jo_files(&self) -> Result<(), AppError> {
        for entry in bundle::jo_managed_files() {
            let path = self.jules_path().join(entry.path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, entry.content)?;
        }
        Ok(())
    }

    /// Detect modified jo-managed files by comparing content hashes.
    pub fn detect_modifications(&self) -> Result<Vec<String>, AppError> {
        let mut modified = Vec::new();
        let managed = jo_managed_map();

        for (path, expected_content) in managed {
            let full_path = self.jules_path().join(path);
            if full_path.exists() {
                let actual_content = fs::read_to_string(&full_path)?;
                if hash_content(&actual_content) != hash_content(expected_content) {
                    modified.push(path.to_string());
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

    /// Check if a role exists.
    pub fn role_exists(&self, role_id: &str) -> bool {
        self.role_path(role_id).exists()
    }

    /// Create a role directory with initial files.
    pub fn create_role(&self, role_id: &str) -> Result<(), AppError> {
        let role_dir = self.role_path(role_id);
        fs::create_dir_all(role_dir.join("sessions"))?;

        // Write charter with role_id substitution
        let charter = bundle::role_charter_template().replace("{{role_id}}", role_id);
        fs::write(role_dir.join("charter.md"), charter)?;

        // Write direction with role_id substitution
        let direction = bundle::role_direction_template().replace("{{role_id}}", role_id);
        fs::write(role_dir.join("direction.md"), direction)?;

        Ok(())
    }

    /// Create a session file for a role.
    ///
    /// Returns the path to the created session file.
    pub fn create_session(
        &self,
        role_id: &str,
        date: &str,
        time: &str,
        slug: &str,
    ) -> Result<PathBuf, AppError> {
        let role_dir = self.role_path(role_id);
        let session_dir = role_dir.join("sessions").join(date);
        fs::create_dir_all(&session_dir)?;

        let filename = format!("{}_{}.md", time.replace(':', ""), slug);
        let session_path = session_dir.join(&filename);

        let content = bundle::session_template()
            .replace("{{role_id}}", role_id)
            .replace("{{slug}}", slug)
            .replace("{{date}}", date)
            .replace("{{time}}", time);

        fs::write(&session_path, content)?;

        Ok(session_path)
    }
}

/// Compute a SHA-256 hash of content for comparison.
fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Validate a role identifier.
pub fn is_valid_role_id(id: &str) -> bool {
    !id.is_empty()
        && id.chars().all(|c| c.is_alphanumeric() || c == '-')
        && !id.starts_with('-')
        && !id.ends_with('-')
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
        assert!(ws.jules_path().join(".jo").exists());
        assert!(ws.jules_path().join("org").exists());
        assert!(ws.jules_path().join("roles").exists());
        assert!(ws.jules_path().join("START_HERE.md").exists());
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
    fn detect_modifications_finds_changed_file() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().unwrap();

        // Modify a jo-managed file
        let policy_path = ws.jules_path().join(".jo/policy/contract.md");
        fs::write(&policy_path, "MODIFIED CONTENT").unwrap();

        let mods = ws.detect_modifications().unwrap();
        assert!(mods.contains(&".jo/policy/contract.md".to_string()));
    }

    #[test]
    fn is_valid_role_id_accepts_valid() {
        assert!(is_valid_role_id("value"));
        assert!(is_valid_role_id("quality"));
        assert!(is_valid_role_id("my-role"));
        assert!(is_valid_role_id("role123"));
    }

    #[test]
    fn is_valid_role_id_rejects_invalid() {
        assert!(!is_valid_role_id(""));
        assert!(!is_valid_role_id("-starts"));
        assert!(!is_valid_role_id("ends-"));
        assert!(!is_valid_role_id("has/slash"));
        assert!(!is_valid_role_id("has space"));
    }

    #[test]
    fn create_role_creates_directory_structure() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().unwrap();
        ws.create_role("value").unwrap();

        let role_dir = ws.role_path("value");
        assert!(role_dir.exists());
        assert!(role_dir.join("charter.md").exists());
        assert!(role_dir.join("direction.md").exists());
        assert!(role_dir.join("sessions").exists());
    }
}
