use std::path::{Path, PathBuf};

use crate::domain::Layer;

/// The schemas directory name within `.jules/`.
pub const SCHEMAS_DIR: &str = "schemas";

/// `.jules/schemas/`
pub fn schemas_base_dir(jules_path: &Path) -> PathBuf {
    jules_path.join(SCHEMAS_DIR)
}

/// `.jules/schemas/<layer>/`
pub fn schemas_dir(jules_path: &Path, layer: Layer) -> PathBuf {
    schemas_base_dir(jules_path).join(layer.dir_name())
}

/// `.jules/schemas/<layer>/<filename>`
pub fn schema_file(jules_path: &Path, layer: Layer, filename: &str) -> PathBuf {
    schemas_dir(jules_path, layer).join(filename)
}

/// `.jules/schemas/narrator/changes.yml`
pub fn narrator_change_schema(jules_path: &Path) -> PathBuf {
    schema_file(jules_path, Layer::Narrator, "changes.yml")
}
