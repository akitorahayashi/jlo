use std::path::{Path, PathBuf};

use crate::domain::workstations;

/// `.jlo/config.toml`
pub fn config(root: &Path) -> PathBuf {
    workstations::paths::jlo_dir(root).join("config.toml")
}
