use std::path::{Path, PathBuf};

/// The `.jlo/` control-plane directory name.
pub const JLO_DIR: &str = ".jlo";

/// `.jlo/`
pub fn jlo_dir(root: &Path) -> PathBuf {
    root.join(JLO_DIR)
}

/// `.jlo/workspaces/`
pub fn workspaces_dir(root: &Path) -> PathBuf {
    jlo_dir(root).join("workspaces")
}
