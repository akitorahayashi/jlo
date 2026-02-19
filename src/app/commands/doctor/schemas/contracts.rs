use serde_yaml::Mapping;
use std::path::Path;

use crate::app::commands::doctor::diagnostics::Diagnostics;
use crate::app::commands::doctor::yaml::get_string;
use crate::domain::Layer;

pub fn validate_contracts(
    data: &Mapping,
    path: &Path,
    layer: Layer,
    diagnostics: &mut Diagnostics,
) {
    let layer_value = get_string(data, "layer").unwrap_or_default();
    if layer_value != layer.dir_name() {
        diagnostics.push_error(
            path.display().to_string(),
            format!("layer '{}' does not match directory '{}'", layer_value, layer.dir_name()),
        );
    }

    let prefix = get_string(data, "branch_prefix").unwrap_or_default();
    let layer_slug = layer.dir_name().trim_end_matches('s');
    if !prefix.starts_with(&format!("jules-{}-", layer_slug)) {
        diagnostics.push_error(path.display().to_string(), "branch_prefix is invalid");
    }
}
