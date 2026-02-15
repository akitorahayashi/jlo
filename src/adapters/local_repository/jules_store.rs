//! `JulesStore` and `PromptAssetLoader` implementation for `LocalRepositoryAdapter`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::repository::paths::jules;
use crate::domain::{AppError, JULES_DIR, Layer, PromptAssetLoader, VERSION_FILE};
use crate::ports::{JulesStore, RepositoryFilesystem, ScaffoldFile};

use super::LocalRepositoryAdapter;

impl PromptAssetLoader for LocalRepositoryAdapter {
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
            let layer_dir = jules::layer_dir(&jules_path, layer);
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
    use crate::domain::Layer;
    use crate::ports::{JulesStore, ScaffoldFile};

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
}
