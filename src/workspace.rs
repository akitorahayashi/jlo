//! Workspace operations for `.jules/` directory management.

use std::fs;
use std::path::PathBuf;

use crate::error::AppError;
use crate::layers::Layer;
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

/// A discovered role with its layer and ID.
#[derive(Debug, Clone)]
pub struct DiscoveredRole {
    pub layer: Layer,
    pub id: String,
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
    #[allow(dead_code)]
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

    /// Create the complete `.jules/` directory structure with 4-layer architecture.
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

        // Create layer directories
        for layer in Layer::ALL {
            let layer_dir = self.jules_path().join("roles").join(layer.dir_name());
            fs::create_dir_all(&layer_dir)?;
        }

        // Scaffold all built-in roles under their respective layers
        for role in scaffold::role_definitions() {
            if !self.role_exists_in_layer(role.layer, role.id) {
                self.scaffold_role_in_layer(
                    role.layer,
                    role.id,
                    role.role_yaml,
                    Some(role.prompt_yaml),
                    role.has_notes,
                )?;
            }
        }

        Ok(())
    }

    /// Path to a role directory (layer-nested).
    pub fn role_path_in_layer(&self, layer: Layer, role_id: &str) -> PathBuf {
        self.jules_path().join("roles").join(layer.dir_name()).join(role_id)
    }

    /// Check if a role exists in a specific layer.
    pub fn role_exists_in_layer(&self, layer: Layer, role_id: &str) -> bool {
        if !Self::is_valid_role_id(role_id) {
            return false;
        }
        self.role_path_in_layer(layer, role_id).join("role.yml").exists()
    }

    /// Discover all existing roles across all layers.
    pub fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError> {
        let mut roles = Vec::new();

        for layer in Layer::ALL {
            let layer_dir = self.jules_path().join("roles").join(layer.dir_name());
            if !layer_dir.exists() {
                continue;
            }

            for entry in fs::read_dir(&layer_dir)? {
                let entry = entry?;
                if !entry.path().is_dir() {
                    continue;
                }
                let role_id = entry.file_name().to_string_lossy().to_string();
                if self.role_exists_in_layer(layer, &role_id) {
                    roles.push(DiscoveredRole { layer, id: role_id });
                }
            }
        }

        roles.sort_by(|a, b| {
            let layer_cmp = a.layer.dir_name().cmp(b.layer.dir_name());
            if layer_cmp == std::cmp::Ordering::Equal { a.id.cmp(&b.id) } else { layer_cmp }
        });

        Ok(roles)
    }

    /// Find a role by ID, searching all layers.
    #[allow(dead_code)]
    pub fn find_role(&self, role_id: &str) -> Result<Option<DiscoveredRole>, AppError> {
        let roles = self.discover_roles()?;
        Ok(roles.into_iter().find(|r| r.id == role_id))
    }

    /// Find a role by fuzzy matching (prefix match).
    pub fn find_role_fuzzy(&self, query: &str) -> Result<Option<DiscoveredRole>, AppError> {
        let roles = self.discover_roles()?;

        // Check for exact match first
        if let Some(role) = roles.iter().find(|r| r.id == query) {
            return Ok(Some(role.clone()));
        }

        // Check for layer/role format (e.g., "observers/taxonomy")
        if let Some((layer_part, role_part)) = query.split_once('/')
            && let Some(layer) = Layer::from_dir_name(layer_part)
            && let Some(role) = roles.iter().find(|r| r.layer == layer && r.id == role_part)
        {
            return Ok(Some(role.clone()));
        }

        // Check for prefix match
        let matches: Vec<_> = roles.iter().filter(|r| r.id.starts_with(query)).collect();

        match matches.len() {
            1 => Ok(Some(matches[0].clone())),
            0 => Ok(None),
            _ => Ok(None), // Ambiguous matches
        }
    }

    /// Check if a role_id is valid (no path traversal characters).
    fn is_valid_role_id(role_id: &str) -> bool {
        !role_id.contains('/')
            && !role_id.contains('\\')
            && role_id != "."
            && role_id != ".."
            && !role_id.is_empty()
    }

    /// Scaffold a new role under a specific layer.
    pub fn scaffold_role_in_layer(
        &self,
        layer: Layer,
        role_id: &str,
        role_yaml: &str,
        prompt_yaml: Option<&str>,
        has_notes: bool,
    ) -> Result<(), AppError> {
        if !Self::is_valid_role_id(role_id) {
            return Err(AppError::InvalidRoleId(role_id.to_string()));
        }

        let role_dir = self.role_path_in_layer(layer, role_id);
        fs::create_dir_all(&role_dir)?;
        fs::write(role_dir.join("role.yml"), role_yaml)?;

        if let Some(prompt_content) = prompt_yaml {
            fs::write(role_dir.join("prompt.yml"), prompt_content)?;
        }

        if has_notes {
            let notes_dir = role_dir.join("notes");
            fs::create_dir_all(&notes_dir)?;
            fs::write(notes_dir.join(".gitkeep"), "")?;
        }

        Ok(())
    }

    /// Read the role scheduler prompt (prompt.yml) for a discovered role.
    #[allow(dead_code)]
    pub fn read_role_prompt(&self, role: &DiscoveredRole) -> Result<String, AppError> {
        if !self.exists() {
            return Err(AppError::WorkspaceNotFound);
        }

        let prompt_path = self.role_path_in_layer(role.layer, &role.id).join("prompt.yml");
        if !prompt_path.exists() {
            return Err(AppError::config_error(format!(
                "Role '{}' does not have prompt.yml",
                role.id
            )));
        }

        Ok(fs::read_to_string(prompt_path)?)
    }
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
        assert!(ws.jules_path().join("events/bugs").exists());
        assert!(ws.jules_path().join("events/docs").exists());
        assert!(ws.jules_path().join("events/refacts").exists());
        assert!(ws.jules_path().join("events/tests").exists());
        assert!(ws.jules_path().join("events/updates").exists());
        assert!(ws.jules_path().join("issues").exists());
    }

    #[test]
    fn create_structure_creates_layer_directories() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().expect("create_structure should succeed");

        for layer in Layer::ALL {
            assert!(
                ws.jules_path().join("roles").join(layer.dir_name()).exists(),
                "Layer directory {:?} should exist",
                layer
            );
        }
    }

    #[test]
    fn create_structure_scaffolds_all_builtin_roles_in_layers() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().expect("create_structure should succeed");

        assert!(ws.role_exists_in_layer(Layer::Observers, "taxonomy"));
        assert!(ws.role_exists_in_layer(Layer::Observers, "data_arch"));
        assert!(ws.role_exists_in_layer(Layer::Observers, "qa"));
        assert!(ws.role_exists_in_layer(Layer::Deciders, "triage"));
        assert!(ws.role_exists_in_layer(Layer::Planners, "specifier"));
        assert!(ws.role_exists_in_layer(Layer::Implementers, "executor"));
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
    fn discover_roles_finds_existing() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().unwrap();

        let roles = ws.discover_roles().unwrap();
        assert!(!roles.is_empty());

        let taxonomy = roles.iter().find(|r| r.id == "taxonomy");
        assert!(taxonomy.is_some());
        assert_eq!(taxonomy.unwrap().layer, Layer::Observers);
    }

    #[test]
    fn find_role_fuzzy_matches_prefix() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().unwrap();

        let result = ws.find_role_fuzzy("tax").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "taxonomy");
    }

    #[test]
    fn find_role_fuzzy_matches_layer_prefix() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().unwrap();

        let result = ws.find_role_fuzzy("observers/taxonomy").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "taxonomy");
    }

    #[test]
    fn scaffold_role_in_layer_creates_structure() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().unwrap();

        ws.scaffold_role_in_layer(Layer::Observers, "my-role", "My role config", None, true)
            .unwrap();

        let role_dir = ws.role_path_in_layer(Layer::Observers, "my-role");
        assert!(role_dir.join("role.yml").exists());
        assert!(role_dir.join("notes").exists());
        assert!(role_dir.join("notes/.gitkeep").exists());

        let config = fs::read_to_string(role_dir.join("role.yml")).unwrap();
        assert_eq!(config, "My role config");
    }

    #[test]
    fn scaffold_role_in_layer_with_prompt() {
        let (_dir, ws) = test_workspace();
        ws.create_structure().unwrap();

        ws.scaffold_role_in_layer(
            Layer::Deciders,
            "my-triage",
            "Triage config",
            Some("Triage prompt"),
            false,
        )
        .unwrap();

        let role_dir = ws.role_path_in_layer(Layer::Deciders, "my-triage");
        assert!(role_dir.join("role.yml").exists());
        assert!(role_dir.join("prompt.yml").exists());

        let prompt = fs::read_to_string(role_dir.join("prompt.yml")).unwrap();
        assert_eq!(prompt, "Triage prompt");
    }
}
