//! Test double for `RepositoryFilesystem`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::domain::AppError;
use crate::ports::RepositoryFilesystem;

use super::test_files::TestFiles;

/// In-memory implementation of `RepositoryFilesystem` for unit tests.
#[derive(Clone, Debug)]
pub struct MockRepositoryFs {
    files: TestFiles,
}

impl MockRepositoryFs {
    pub fn new(files: TestFiles) -> Self {
        Self { files }
    }
}

impl RepositoryFilesystem for MockRepositoryFs {
    fn read_file(&self, path: &str) -> Result<String, AppError> {
        self.files.files.lock().unwrap().get(path).cloned().ok_or_else(|| {
            AppError::from(std::io::Error::new(std::io::ErrorKind::NotFound, "Mock file not found"))
        })
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), AppError> {
        self.files.files.lock().unwrap().insert(path.to_string(), content.to_string());
        Ok(())
    }

    fn remove_file(&self, path: &str) -> Result<(), AppError> {
        self.files.files.lock().unwrap().remove(path);
        Ok(())
    }

    fn remove_dir_all(&self, path: &str) -> Result<(), AppError> {
        let prefix = if path.ends_with('/') { path.to_string() } else { format!("{}/", path) };
        self.files.files.lock().unwrap().retain(|key, _| !key.starts_with(&prefix));
        Ok(())
    }

    fn list_dir(&self, path: &str) -> Result<Vec<PathBuf>, AppError> {
        let prefix = if path.ends_with('/') { path.to_string() } else { format!("{}/", path) };
        let path_obj = Path::new(path);
        let mut results = HashSet::new();

        for key in self.files.files.lock().unwrap().keys() {
            if key.starts_with(&prefix) {
                let suffix = &key[prefix.len()..];
                if let Some(slash_idx) = suffix.find('/') {
                    let dir_name = &suffix[..slash_idx];
                    results.insert(path_obj.join(dir_name));
                } else {
                    results.insert(PathBuf::from(key));
                }
            }
        }

        let mut results_vec: Vec<PathBuf> = results.into_iter().collect();
        results_vec.sort();
        Ok(results_vec)
    }

    fn set_executable(&self, _path: &str) -> Result<(), AppError> {
        Ok(())
    }

    fn file_exists(&self, path: &str) -> bool {
        let files = self.files.files.lock().unwrap();
        if files.contains_key(path) {
            return true;
        }
        let prefix = if path.ends_with('/') { path.to_string() } else { format!("{}/", path) };
        files.keys().any(|k| k.starts_with(&prefix))
    }

    fn is_dir(&self, path: &str) -> bool {
        let prefix = if path.ends_with('/') { path.to_string() } else { format!("{}/", path) };
        self.files.files.lock().unwrap().keys().any(|k| k.starts_with(&prefix))
    }

    fn create_dir_all(&self, _path: &str) -> Result<(), AppError> {
        Ok(())
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    fn canonicalize(&self, path: &str) -> Result<PathBuf, AppError> {
        Ok(PathBuf::from(path))
    }
}
