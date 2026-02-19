use std::path::{Path, PathBuf};

use crate::domain::jlo_paths;

/// `.jlo/config.toml`
pub fn config(root: &Path) -> PathBuf {
    jlo_paths::jlo_dir(root).join("config.toml")
}
