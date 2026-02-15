use std::path::{Path, PathBuf};

/// The `.jules/` runtime directory name.
pub const JULES_DIR: &str = ".jules";

/// The `.jlo/` control-plane directory name.
pub const JLO_DIR: &str = ".jlo";

/// The version marker file name.
pub const VERSION_FILE: &str = ".jlo-version";

/// `.jules/`
pub fn jules_dir(root: &Path) -> PathBuf {
    root.join(JULES_DIR)
}

/// `.jlo/`
pub fn jlo_dir(root: &Path) -> PathBuf {
    root.join(JLO_DIR)
}

/// `.jules/JULES.md`
pub fn jules_readme(root: &Path) -> PathBuf {
    jules_dir(root).join("JULES.md")
}

/// `.jules/README.md`
pub fn project_readme(root: &Path) -> PathBuf {
    jules_dir(root).join("README.md")
}

/// `.jules/.jlo-version`
pub fn version_file(root: &Path) -> PathBuf {
    jules_dir(root).join(VERSION_FILE)
}

/// `.jules/github-labels.json`
pub fn github_labels(jules_path: &Path) -> PathBuf {
    jules_path.join("github-labels.json")
}

/// `.jules/workstations/`
pub fn workstations_dir(jules_path: &Path) -> PathBuf {
    jules_path.join("workstations")
}

/// `.jules/workstations/<role>/`
pub fn workstation_dir(jules_path: &Path, role: &str) -> PathBuf {
    workstations_dir(jules_path).join(role)
}

/// `.jules/workstations/<role>/perspective.yml`
pub fn workstation_perspective(jules_path: &Path, role: &str) -> PathBuf {
    workstation_dir(jules_path, role).join("perspective.yml")
}
