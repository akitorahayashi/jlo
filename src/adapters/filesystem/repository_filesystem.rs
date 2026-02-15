//! `RepositoryFilesystemPort` implementation for `FilesystemStore`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::AppError;
use crate::ports::RepositoryFilesystemPort;

use super::FilesystemStore;

impl RepositoryFilesystemPort for FilesystemStore {
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

    fn remove_dir_all(&self, path: &str) -> Result<(), AppError> {
        let full_path = self.resolve_path(path);
        self.validate_path_within_root(&full_path)?;
        if full_path.exists() {
            fs::remove_dir_all(full_path).map_err(AppError::from)?;
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
        if self.validate_path_within_root(&full_path).is_err() {
            return false;
        }
        full_path.exists()
    }

    fn is_dir(&self, path: &str) -> bool {
        let full_path = self.resolve_path(path);
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
        let p =
            if Path::new(path).is_absolute() { PathBuf::from(path) } else { self.root.join(path) };
        fs::canonicalize(p).map_err(AppError::from)
    }
}
