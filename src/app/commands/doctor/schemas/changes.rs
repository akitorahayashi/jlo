use serde_yaml::Mapping;
use std::path::Path;

use crate::app::commands::doctor::diagnostics::Diagnostics;
use crate::app::commands::doctor::yaml::{
    ensure_int, ensure_non_empty_sequence, ensure_non_empty_string, load_yaml_mapping,
};

/// Validate .jules/exchange/changes.yml schema.
pub fn validate_changes_file(path: &Path, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };

    validate_changes_data(&data, path, diagnostics);
}

pub fn validate_changes_data(data: &Mapping, path: &Path, diagnostics: &mut Diagnostics) {
    // Required fields
    ensure_int(data, path, "schema_version", diagnostics, Some(1));
    ensure_non_empty_string(data, path, "created_at", diagnostics);

    // Validate summaries sequence
    ensure_non_empty_sequence(data, path, "summaries", diagnostics);
}
