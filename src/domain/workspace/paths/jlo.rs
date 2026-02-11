//! `.jlo/` control-plane paths.

use std::path::{Path, PathBuf};

use crate::domain::workspace::layer::Layer;
use crate::domain::workspace::manifest::MANIFEST_FILENAME;

// ── Top-level files ────────────────────────────────────────────────────

/// `.jlo/config.toml`
pub fn config(root: &Path) -> PathBuf {
    root.join(super::JLO_DIR).join("config.toml")
}

/// `.jlo/scheduled.toml`
pub fn schedule(root: &Path) -> PathBuf {
    root.join(super::JLO_DIR).join("scheduled.toml")
}

/// `.jlo/.jlo-managed.yml` — relative path string.
pub fn manifest_relative() -> String {
    format!("{}/{}", super::JLO_DIR, MANIFEST_FILENAME)
}

// ── Roles ──────────────────────────────────────────────────────────────

/// `.jlo/roles/`
pub fn roles_dir(root: &Path) -> PathBuf {
    root.join(super::JLO_DIR).join("roles")
}

/// `.jlo/roles/<layer>/`
pub fn layer_dir(root: &Path, layer: Layer) -> PathBuf {
    roles_dir(root).join(layer.dir_name())
}

/// `.jlo/roles/<layer>/<role>/`
pub fn role_dir(root: &Path, layer: Layer, role: &str) -> PathBuf {
    layer_dir(root, layer).join(role)
}

/// `.jlo/roles/<layer>/<role>/role.yml`
pub fn role_yml(root: &Path, layer: Layer, role: &str) -> PathBuf {
    role_dir(root, layer, role).join("role.yml")
}

// ── Relative path helpers for WorkspaceStore string-based operations ───

/// `.jlo/scheduled.toml` — relative path string.
pub fn schedule_relative() -> &'static str {
    ".jlo/scheduled.toml"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_paths() {
        let root = Path::new("/ws");
        assert_eq!(config(root), PathBuf::from("/ws/.jlo/config.toml"));
        assert_eq!(schedule(root), PathBuf::from("/ws/.jlo/scheduled.toml"));
    }

    #[test]
    fn role_paths() {
        let root = Path::new("/ws");
        assert_eq!(
            role_yml(root, Layer::Observers, "taxonomy"),
            PathBuf::from("/ws/.jlo/roles/observers/taxonomy/role.yml")
        );
    }

    #[test]
    fn relative_strings() {
        assert_eq!(schedule_relative(), ".jlo/scheduled.toml");
        assert_eq!(manifest_relative(), ".jlo/.jlo-managed.yml");
    }
}
