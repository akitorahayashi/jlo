use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::workspace::paths::{jlo, jules};
use crate::domain::{AppError, JLO_DIR, JULES_DIR, Layer, PromptAssetLoader, RoleId, VERSION_FILE};
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
}

impl PromptAssetLoader for FilesystemWorkspaceStore {
    fn read_asset(&self, path: &Path) -> std::io::Result<String> {
        fs::read_to_string(path)
    }

    fn asset_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn ensure_asset_dir(&self, path: &Path) -> std::io::Result<()> {
        fs::create_dir_all(path)
    }

    fn copy_asset(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        fs::copy(from, to)
    }
}

impl WorkspaceStore for FilesystemWorkspaceStore {
    fn exists(&self) -> bool {
        self.jules_path().exists()
    }

    fn jlo_exists(&self) -> bool {
        self.jlo_path().exists()
    }

    fn jules_path(&self) -> PathBuf {
        self.root.join(JULES_DIR)
    }

    fn jlo_path(&self) -> PathBuf {
        self.root.join(JLO_DIR)
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
            let layer_dir = jules::layer_dir(&self.jules_path(), layer);
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

    fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError> {
        let mut roles = Vec::new();

        for layer in Layer::ALL {
            if layer.is_single_role() {
                continue;
            }
            // Convention: .jlo/roles/<layer>/<role>/ (see also role_path)
            let layer_dir = jlo::layer_dir(&self.root, layer);
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
                    && entry.path().join("role.yml").exists()
                {
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

    fn find_role_fuzzy(&self, query: &str) -> Result<Option<DiscoveredRole>, AppError> {
        let roles = self.discover_roles()?;

        // Check for exact match first
        if let Some(role) = roles.iter().find(|r| r.id.as_str() == query) {
            return Ok(Some(role.clone()));
        }

        // Check for layer/role format (e.g., "observers/taxonomy")
        if let Some((layer_part, role_part)) = query.split_once('/')
            && let Some(layer) = Layer::from_dir_name(layer_part)
            && let Some(role) =
                roles.iter().find(|r| r.layer == layer && r.id.as_str() == role_part)
        {
            return Ok(Some(role.clone()));
        }

        // Check for prefix match
        let matches: Vec<_> = roles.iter().filter(|r| r.id.as_str().starts_with(query)).collect();

        match matches.len() {
            1 => Ok(Some(matches[0].clone())),
            0 => Ok(None),
            _ => Ok(None), // Ambiguous matches
        }
    }

    fn role_path(&self, role: &DiscoveredRole) -> Option<PathBuf> {
        // Convention: .jlo/roles/<layer>/roles/<id> (see also discover_roles)
        let path = jlo::layer_dir(&self.root, role.layer).join("roles").join(role.id.as_str());
        if path.exists() { Some(path) } else { None }
    }

    fn read_file(&self, path: &str) -> Result<String, AppError> {
        let full_path = self.resolve_path(path);
        self.validate_path_within_root(&full_path)?;
        fs::read_to_string(full_path).map_err(AppError::from)
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), AppError> {
        let full_path = self.resolve_path(path);
        self.validate_path_within_root(&full_path)?;
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(AppError::from)?;
        }
        fs::write(full_path, content).map_err(AppError::from)
    }

    fn remove_file(&self, path: &str) -> Result<(), AppError> {
        let full_path = self.resolve_path(path);
        self.validate_path_within_root(&full_path)?;
        if full_path.exists() {
            fs::remove_file(full_path).map_err(AppError::from)?;
        }
        Ok(())
    }

    fn list_dir(&self, path: &str) -> Result<Vec<PathBuf>, AppError> {
        let full_path = self.resolve_path(path);
        self.validate_path_within_root(&full_path)?;
        let entries = fs::read_dir(full_path).map_err(AppError::from)?;
        let mut paths = Vec::new();
        for entry in entries {
            let entry = entry.map_err(AppError::from)?;
            paths.push(entry.path());
        }
        // sort for determinism
        paths.sort();
        Ok(paths)
    }

    fn set_executable(&self, path: &str) -> Result<(), AppError> {
        let full_path = self.resolve_path(path);
        self.validate_path_within_root(&full_path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&full_path).map_err(AppError::from)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&full_path, perms).map_err(AppError::from)?;
        }
        Ok(())
    }

    fn file_exists(&self, path: &str) -> bool {
        let full_path = self.resolve_path(path);
        // For existence checks, allow traversal detection to fail silently
        if self.validate_path_within_root(&full_path).is_err() {
            return false;
        }
        full_path.exists()
    }

    fn is_dir(&self, path: &str) -> bool {
        let full_path = self.resolve_path(path);
        // For existence checks, allow traversal detection to fail silently
        if self.validate_path_within_root(&full_path).is_err() {
            return false;
        }
        full_path.is_dir()
    }

    fn create_dir_all(&self, path: &str) -> Result<(), AppError> {
        let full_path = self.resolve_path(path);
        self.validate_path_within_root(&full_path)?;
        fs::create_dir_all(full_path).map_err(AppError::from)
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    fn canonicalize(&self, path: &str) -> Result<PathBuf, AppError> {
        // Since we assume CWD is root, passing path (relative or absolute) to fs::canonicalize should work.
        // But if path is relative, fs::canonicalize resolves against CWD.
        // We want to resolve against self.root?
        // If self.root is absolute, and path is relative...
        let p = if PathBuf::from(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.root.join(path)
        };
        fs::canonicalize(p).map_err(AppError::from)
    }
}

// Private helper methods for FilesystemWorkspaceStore
impl FilesystemWorkspaceStore {
    /// Validates that a path (or its nearest existing ancestor) is within the workspace root.
    /// Validates that a path is within the workspace root.
    ///
    /// This implementation uses logical path normalization to resolve `..` and `.` components
    /// without relying on the filesystem (unlike `fs::canonicalize`). This ensures that
    /// even if intermediate directories don't exist, we can still correctly validation
    /// that the final path would lie within the root.
    fn validate_path_within_root(&self, path: &Path) -> Result<(), AppError> {
        let full_path = if path.is_absolute() { path.to_path_buf() } else { self.root.join(path) };

        let normalized_path = normalize_path(&full_path);
        let normalized_root = normalize_path(&self.root);

        if !normalized_path.starts_with(&normalized_root) {
            return Err(AppError::PathTraversal(path.display().to_string()));
        }

        Ok(())
    }
}

/// Normalize path by resolving `.` and `..` components logically.
/// This does not access the filesystem.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(std::path::Component::RootDir) = components.peek() {
        components.next();
        PathBuf::from("/")
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            std::path::Component::Prefix(..) => {
                // Keep prefix as is (e.g., C:\ on Windows)
                ret.push(component.as_os_str());
            }
            std::path::Component::RootDir => {
                // Should have been handled at the start, but just in case
                ret.push(component.as_os_str());
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                ret.pop();
            }
            std::path::Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
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
    fn discover_roles_finds_and_sorts_roles() {
        let (_dir, ws) = test_workspace();
        ws.create_structure(&[]).unwrap();

        // Create role directories directly (scaffold_role_in_layer removed with template retirement)
        let obs_dir = ws.jlo_path().join("roles/observers/taxonomy");
        fs::create_dir_all(&obs_dir).unwrap();
        fs::write(obs_dir.join("role.yml"), "role: taxonomy\nlayer: observers").unwrap();

        let inn_dir = ws.jlo_path().join("roles/innovators/screener");
        fs::create_dir_all(&inn_dir).unwrap();
        fs::write(inn_dir.join("role.yml"), "role: screener\nlayer: innovators").unwrap();

        // Note: Planners is a single-role layer, so we don't create roles in it

        let roles = ws.discover_roles().unwrap();

        assert_eq!(roles.len(), 2);
        // Sort order is by dir_name: innovators, observers
        assert_eq!(roles[0].layer, Layer::Innovators);
        assert_eq!(roles[0].id.as_str(), "screener");

        assert_eq!(roles[1].layer, Layer::Observers);
        assert_eq!(roles[1].id.as_str(), "taxonomy");
    }

    #[test]
    fn find_role_fuzzy_matches() {
        let (_dir, ws) = test_workspace();
        ws.create_structure(&[]).unwrap();

        let obs_dir = ws.jlo_path().join("roles/observers/taxonomy");
        fs::create_dir_all(&obs_dir).unwrap();
        fs::write(obs_dir.join("role.yml"), "role: taxonomy\nlayer: observers").unwrap();

        let inn_dir = ws.jlo_path().join("roles/innovators/taxman");
        fs::create_dir_all(&inn_dir).unwrap();
        fs::write(inn_dir.join("role.yml"), "role: taxman\nlayer: innovators").unwrap();

        // Exact match
        let found = ws.find_role_fuzzy("taxonomy").unwrap().unwrap();
        assert_eq!(found.layer, Layer::Observers);
        assert_eq!(found.id.as_str(), "taxonomy");

        // Layer/Role match
        let found = ws.find_role_fuzzy("innovators/taxman").unwrap().unwrap();
        assert_eq!(found.layer, Layer::Innovators);
        assert_eq!(found.id.as_str(), "taxman");

        // Prefix match (unique)
        let found = ws.find_role_fuzzy("taxo").unwrap().unwrap();
        assert_eq!(found.id.as_str(), "taxonomy");

        // Prefix match (ambiguous) - "tax" matches "taxonomy" and "taxman"
        let found = ws.find_role_fuzzy("tax").unwrap();
        assert!(found.is_none());

        // No match
        let found = ws.find_role_fuzzy("nonexistent").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn validate_path_prevents_traversal_with_nonexistent_components() {
        let (_dir, ws) = test_workspace();
        // "nonexistent/../../etc/passwd" style attack

        // Case 1: Simple escape
        let bad_path = "../result.txt";
        let result = ws.validate_path_within_root(&ws.resolve_path(bad_path));
        assert!(result.is_err(), "Should detect simple traversal");

        // Case 2: Escape with non-existent intermediate
        // root/nonexistent/../../outside
        let bad_path_complex = "nonexistent/../../outside_result.txt";
        let result = ws.validate_path_within_root(&ws.resolve_path(bad_path_complex));
        assert!(
            result.is_err(),
            "Should detect traversal even if 'nonexistent' components don't exist"
        );

        // Case 3: Valid path with .. that stays inside
        let good_path_complex = "subdir/../result.txt";
        let result = ws.validate_path_within_root(&ws.resolve_path(good_path_complex));
        assert!(result.is_ok(), "Should allow .. that stays within root: {:?}", result.err());
    }
}
