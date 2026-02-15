use std::path::{Path, PathBuf};

use crate::domain::Layer;

/// The layers directory name.
pub const LAYERS_DIR: &str = "layers";

/// `.jules/layers/`
pub fn layers_dir(jules_path: &Path) -> PathBuf {
    jules_path.join(LAYERS_DIR)
}

/// `.jules/layers/<layer>/`
pub fn layer_dir(jules_path: &Path, layer: Layer) -> PathBuf {
    layers_dir(jules_path).join(layer.dir_name())
}

/// `.jules/layers/<layer>/<layer>_prompt.j2`
pub fn prompt_template(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join(layer.prompt_template_name())
}

/// `.jules/layers/<layer>/contracts.yml`
pub fn contracts(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join("contracts.yml")
}

/// `.jules/layers/<layer>/schemas/`
pub fn schemas_dir(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join("schemas")
}

/// `.jules/layers/<layer>/tasks/`
pub fn tasks_dir(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join("tasks")
}

/// `.jules/layers/<layer>/schemas/<filename>`
pub fn schema_file(jules_path: &Path, layer: Layer, filename: &str) -> PathBuf {
    schemas_dir(jules_path, layer).join(filename)
}

/// `.jules/layers/<layer>/roles/`
pub fn layer_roles_container(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join("roles")
}

/// `.jules/layers/narrator/schemas/changes.yml`
pub fn narrator_change_schema(jules_path: &Path) -> PathBuf {
    schema_file(jules_path, Layer::Narrator, "changes.yml")
}
