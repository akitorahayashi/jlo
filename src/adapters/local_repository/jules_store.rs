//! `JulesStore` and `PromptAssetLoader` implementation for `LocalRepositoryAdapter`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::adapters::catalogs::builtin_role_assets::{
    load_builtin_role_catalog, read_builtin_role_file,
};
use crate::domain::{AppError, JULES_DIR, Layer, PromptAssetLoader, VERSION_FILE};
use crate::ports::{JulesStore, RepositoryFilesystem, ScaffoldFile};

use super::LocalRepositoryAdapter;

impl PromptAssetLoader for LocalRepositoryAdapter {
    fn read_asset(&self, path: &Path) -> std::io::Result<String> {
        match fs::read_to_string(path) {
            Ok(content) => Ok(content),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound
                    && let Some((layer, role)) = builtin_role_from_control_path(path)
                    && let Some(content) = read_embedded_builtin_role(&layer, &role)
                {
                    return Ok(content);
                }
                Err(err)
            }
        }
    }

    fn asset_exists(&self, path: &Path) -> bool {
        path.exists()
            || builtin_role_from_control_path(path)
                .is_some_and(|(layer, role)| read_embedded_builtin_role(&layer, &role).is_some())
    }

    fn ensure_asset_dir(&self, path: &Path) -> std::io::Result<()> {
        fs::create_dir_all(path)
    }

    fn copy_asset(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        fs::copy(from, to)
    }
}

fn builtin_role_from_control_path(path: &Path) -> Option<(String, String)> {
    let components =
        path.components().map(|c| c.as_os_str().to_str()).collect::<Option<Vec<_>>>()?;

    // <root>/.jlo/roles/<layer>/<role>/role.yml
    if components.len() < 5 {
        return None;
    }
    let n = components.len();
    if components[n - 5] != ".jlo"
        || components[n - 4] != "roles"
        || components[n - 1] != "role.yml"
    {
        return None;
    }

    let layer = components[n - 3];
    let role = components[n - 2];
    if !matches!(layer, "observers" | "innovators") {
        return None;
    }

    // Builtin assets are stored by <layer>/<category>/<role>/role.yml, so we need to
    // probe all categories. Catalog lookup remains the source of truth for layer/role validity.
    // Here we only return the layer/role pair; caller resolves through catalog-aware loader.
    Some((layer.to_string(), role.to_string()))
}

fn read_embedded_builtin_role(layer: &str, role: &str) -> Option<String> {
    let catalog = load_builtin_role_catalog().ok()?;
    let entry = catalog
        .into_iter()
        .find(|entry| entry.layer.dir_name() == layer && entry.name.as_str() == role)?;
    read_builtin_role_file(&entry.path).ok()
}

impl JulesStore for LocalRepositoryAdapter {
    fn jules_exists(&self) -> bool {
        self.jules_path().exists()
    }

    fn jules_path(&self) -> PathBuf {
        self.root.join(JULES_DIR)
    }

    fn create_structure(&self, scaffold_files: &[ScaffoldFile]) -> Result<(), AppError> {
        let jules_path = self.root.join(JULES_DIR);
        fs::create_dir_all(&jules_path)?;

        for entry in scaffold_files {
            let path = self.root.join(&entry.path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, &entry.content)?;
        }

        // Create layer directories
        for layer in Layer::ALL {
            let layer_dir = crate::domain::layers::paths::layer_dir(&jules_path, layer);
            fs::create_dir_all(&layer_dir)?;
        }

        Ok(())
    }

    fn jules_write_version(&self, version: &str) -> Result<(), AppError> {
        let path = format!("{}/{}", JULES_DIR, VERSION_FILE);
        self.write_file(&path, &format!("{}\n", version))
    }

    fn jules_read_version(&self) -> Result<Option<String>, AppError> {
        let path = format!("{}/{}", JULES_DIR, VERSION_FILE);
        if !self.file_exists(&path) {
            return Ok(None);
        }
        let content = self.read_file(&path)?;
        Ok(Some(content.trim().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::test_store;
    use crate::domain::{Layer, PromptAssetLoader};
    use crate::ports::{JulesStore, ScaffoldFile};
    use std::fs;

    #[test]
    fn create_structure_creates_directories() {
        let (_dir, store) = test_store();
        let files = vec![ScaffoldFile {
            path: ".jules/README.md".to_string(),
            content: "# Test".to_string(),
        }];
        store.create_structure(&files).unwrap();

        assert!(store.jules_path().exists());
        assert!(store.jules_path().join("layers").exists());
        assert!(store.jules_path().join("README.md").exists());
    }

    #[test]
    fn create_structure_creates_layer_directories() {
        let (_dir, store) = test_store();
        store.create_structure(&[]).unwrap();

        for layer in Layer::ALL {
            assert!(
                store.jules_path().join("layers").join(layer.dir_name()).exists(),
                "Layer directory {:?} should exist",
                layer
            );
        }
    }

    #[test]
    fn version_roundtrip() {
        let (_dir, store) = test_store();
        store.create_structure(&[]).unwrap();

        store.jules_write_version("0.1.0").unwrap();
        let version = store.jules_read_version().unwrap();
        assert_eq!(version, Some("0.1.0".to_string()));
    }

    #[test]
    fn prompt_loader_reads_embedded_builtin_when_custom_role_is_missing() {
        let (_dir, store) = test_store();
        let role_path = store.root.join(".jlo/roles/observers/taxonomy/role.yml");

        assert!(store.asset_exists(&role_path));
        let content = store.read_asset(&role_path).expect("embedded builtin role should resolve");
        assert!(content.contains("role: taxonomy"));
    }

    #[test]
    fn prompt_loader_prefers_custom_role_over_embedded_builtin() {
        let (_dir, store) = test_store();
        let role_path = store.root.join(".jlo/roles/observers/taxonomy/role.yml");
        if let Some(parent) = role_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&role_path, "role: taxonomy\nlayer: observers\nprofile:\n  focus: custom\n")
            .unwrap();

        let content = store.read_asset(&role_path).expect("custom role should resolve");
        assert!(content.contains("focus: custom"));
    }
}
