use crate::config::Config;
use crate::error::AppError;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub(crate) trait Storage {
    fn add_item(&self, id: &str, content: &str) -> Result<(), AppError>;
    fn list_items(&self) -> Result<Vec<String>, AppError>;
    fn delete_item(&self, id: &str) -> Result<(), AppError>;
}

#[derive(Debug, Clone)]
pub(crate) struct FilesystemStorage {
    root_path: PathBuf,
}

impl FilesystemStorage {
    /// Create a new storage with the given configuration.
    pub fn new(config: &Config) -> Self {
        Self { root_path: config.storage_path.clone() }
    }

    /// Create storage with default configuration.
    pub fn new_default() -> Result<Self, AppError> {
        let config = Config::new_default()?;
        Ok(Self::new(&config))
    }

    fn ensure_valid_id(&self, id: &str) -> Result<(), AppError> {
        if Self::is_id_valid(id) {
            Ok(())
        } else {
            Err(AppError::config_error(format!("invalid item identifier: {id}")))
        }
    }

    fn is_id_valid(id: &str) -> bool {
        !id.is_empty()
            && id.chars().all(|c| c.is_alphanumeric() || c == '-')
            && Path::new(id).components().all(|c| matches!(c, Component::Normal(_)))
    }

    fn item_dir(&self, id: &str) -> PathBuf {
        self.root_path.join(id)
    }

    fn item_file(&self, id: &str) -> PathBuf {
        self.item_dir(id).join("item.txt")
    }
}

impl Storage for FilesystemStorage {
    fn add_item(&self, id: &str, content: &str) -> Result<(), AppError> {
        self.ensure_valid_id(id)?;
        let directory = self.item_dir(id);
        fs::create_dir_all(&directory)?;
        fs::write(self.item_file(id), content)?;
        Ok(())
    }

    fn list_items(&self) -> Result<Vec<String>, AppError> {
        if !self.root_path.exists() {
            return Ok(Vec::new());
        }

        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.root_path)? {
            let entry = entry?;
            if entry.path().is_dir()
                && let Some(name) = entry.file_name().to_str()
            {
                ids.push(name.to_string());
            }
        }

        ids.sort();
        Ok(ids)
    }

    fn delete_item(&self, id: &str) -> Result<(), AppError> {
        self.ensure_valid_id(id)?;
        let directory = self.item_dir(id);
        if !directory.exists() {
            return Err(AppError::ItemNotFound(id.to_string()));
        }
        fs::remove_dir_all(directory)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    struct TestContext {
        root: TempDir,
    }

    impl TestContext {
        fn new() -> Self {
            let root = TempDir::new().expect("failed to create temp dir");
            Self { root }
        }

        fn storage(&self) -> FilesystemStorage {
            let config = Config::with_path(self.storage_root());
            FilesystemStorage::new(&config)
        }

        fn storage_root(&self) -> PathBuf {
            self.root.path().join(".config").join("rs-cli-tmpl")
        }
    }

    #[test]
    fn add_item_persists_contents() {
        let ctx = TestContext::new();
        let storage = ctx.storage();

        storage.add_item("demo", "example content").expect("add_item should succeed");

        let saved = ctx.storage_root().join("demo").join("item.txt");
        let content = fs::read_to_string(saved).expect("failed to read saved item");
        assert_eq!(content, "example content");
    }

    #[test]
    fn list_items_returns_all_ids() {
        let ctx = TestContext::new();
        let storage = ctx.storage();

        storage.add_item("first", "one").unwrap();
        storage.add_item("second", "two").unwrap();

        let mut items = storage.list_items().expect("list_items succeeds");
        items.sort();
        assert_eq!(items, vec!["first", "second"]);
    }

    #[test]
    fn delete_item_removes_directory() {
        let ctx = TestContext::new();
        let storage = ctx.storage();

        storage.add_item("temp", "data").unwrap();
        storage.delete_item("temp").expect("delete succeeds");

        assert!(!ctx.storage_root().join("temp").exists());
    }

    #[test]
    fn delete_item_fails_if_not_exists() {
        let ctx = TestContext::new();
        let storage = ctx.storage();

        let result = storage.delete_item("nonexistent");
        assert!(matches!(result, Err(AppError::ItemNotFound(ref id)) if id == "nonexistent"));
    }
}
