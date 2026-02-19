use std::path::{Path, PathBuf};

use crate::domain::{Layer, jlo_paths};

/// The role definition file name.
pub const ROLE_FILENAME: &str = "role.yml";

/// `.jlo/roles/`
pub fn roles_dir(root: &Path) -> PathBuf {
    jlo_paths::jlo_dir(root).join("roles")
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
    role_dir(root, layer, role).join(ROLE_FILENAME)
}
