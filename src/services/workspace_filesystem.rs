use std::fs;
use std::path::PathBuf;

use crate::domain::{AppError, JULES_DIR, Layer, RoleId, VERSION_FILE};
use crate::ports::{DiscoveredRole, ScaffoldFile, WorkspaceStore};

/// Filesystem-based workspace store implementation.
#[derive(Debug, Clone)]
pub struct FilesystemWorkspaceStore {
    root: PathBuf,
}

impl FilesystemWorkspaceStore {
    /// Create a workspace store for the given root directory.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Create a workspace store for the current directory.
    pub fn current() -> Result<Self, AppError> {
        let cwd = std::env::current_dir()?;
        Ok(Self::new(cwd))
    }

    fn version_path(&self) -> PathBuf {
        self.jules_path().join(VERSION_FILE)
    }

    fn role_path_in_layer(&self, layer: Layer, role_id: &str) -> PathBuf {
        self.jules_path().join("roles").join(layer.dir_name()).join(role_id)
    }
}

impl WorkspaceStore for FilesystemWorkspaceStore {
    fn exists(&self) -> bool {
        self.jules_path().exists()
    }

    fn jules_path(&self) -> PathBuf {
        self.root.join(JULES_DIR)
    }

    fn create_structure(&self, scaffold_files: &[ScaffoldFile]) -> Result<(), AppError> {
        fs::create_dir_all(self.jules_path())?;

        // Write scaffold files
        for entry in scaffold_files {
            let path = self.root.join(&entry.path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, &entry.content)?;
        }

        // Create layer directories
        for layer in Layer::ALL {
            let layer_dir = self.jules_path().join("roles").join(layer.dir_name());
            fs::create_dir_all(&layer_dir)?;
        }

        Ok(())
    }

    fn write_version(&self, version: &str) -> Result<(), AppError> {
        fs::write(self.version_path(), format!("{}\n", version))?;
        Ok(())
    }

    fn read_version(&self) -> Result<Option<String>, AppError> {
        let path = self.version_path();
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        Ok(Some(content.trim().to_string()))
    }

    fn role_exists_in_layer(&self, layer: Layer, role_id: &RoleId) -> bool {
        self.role_path_in_layer(layer, role_id.as_str()).join("role.yml").exists()
    }

    fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError> {
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
                let role_id_str = entry.file_name().to_string_lossy().to_string();
                if let Ok(role_id) = RoleId::new(&role_id_str)
                    && self.role_exists_in_layer(layer, &role_id)
                {
                    roles.push(DiscoveredRole { layer, id: role_id_str });
                }
            }
        }

        roles.sort_by(|a, b| {
            let layer_cmp = a.layer.dir_name().cmp(b.layer.dir_name());
            if layer_cmp == std::cmp::Ordering::Equal { a.id.cmp(&b.id) } else { layer_cmp }
        });

        Ok(roles)
    }

    fn find_role_fuzzy(&self, query: &str) -> Result<Option<DiscoveredRole>, AppError> {
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

    fn scaffold_role_in_layer(
        &self,
        layer: Layer,
        role_id: &RoleId,
        role_yaml: &str,
        prompt_yaml: Option<&str>,
        has_notes: bool,
    ) -> Result<(), AppError> {
        let role_dir = self.role_path_in_layer(layer, role_id.as_str());
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_workspace() -> (TempDir, FilesystemWorkspaceStore) {
        let dir = TempDir::new().expect("failed to create temp dir");
        let ws = FilesystemWorkspaceStore::new(dir.path().to_path_buf());
        (dir, ws)
    }

    #[test]
    fn workspace_paths_are_correct() {
        let (_dir, ws) = test_workspace();
        assert!(ws.jules_path().ends_with(".jules"));
    }

    #[test]
    fn create_structure_creates_directories() {
        let (_dir, ws) = test_workspace();
        let files = vec![ScaffoldFile {
            path: ".jules/README.md".to_string(),
            content: "# Test".to_string(),
        }];
        ws.create_structure(&files).expect("create_structure should succeed");

        assert!(ws.jules_path().exists());
        assert!(ws.jules_path().join("roles").exists());
        assert!(ws.jules_path().join("README.md").exists());
    }

    #[test]
    fn create_structure_creates_layer_directories() {
        let (_dir, ws) = test_workspace();
        ws.create_structure(&[]).expect("create_structure should succeed");

        for layer in Layer::ALL {
            assert!(
                ws.jules_path().join("roles").join(layer.dir_name()).exists(),
                "Layer directory {:?} should exist",
                layer
            );
        }
    }

    #[test]
    fn version_roundtrip() {
        let (_dir, ws) = test_workspace();
        ws.create_structure(&[]).unwrap();

        ws.write_version("0.1.0").unwrap();
        let version = ws.read_version().unwrap();
        assert_eq!(version, Some("0.1.0".to_string()));
    }

    #[test]
    fn scaffold_role_in_layer_creates_structure() {
        let (_dir, ws) = test_workspace();
        ws.create_structure(&[]).unwrap();

        let role_id = RoleId::new("my-role").unwrap();
        ws.scaffold_role_in_layer(Layer::Observers, &role_id, "My role config", None, true)
            .unwrap();

        let role_dir = ws.jules_path().join("roles/observers/my-role");
        assert!(role_dir.join("role.yml").exists());
        assert!(role_dir.join("notes").exists());
        assert!(role_dir.join("notes/.gitkeep").exists());
    }
}
