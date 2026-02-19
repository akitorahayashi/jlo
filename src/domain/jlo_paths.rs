use std::path::{Path, PathBuf};

/// The `.jlo/` control-plane directory name.
pub const JLO_DIR: &str = ".jlo";

/// `.jlo/`
pub fn jlo_dir(root: &Path) -> PathBuf {
    root.join(JLO_DIR)
}
