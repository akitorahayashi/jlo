use serde_yaml::Mapping;
use std::path::Path;

use crate::app::commands::doctor::diagnostics::Diagnostics;
use crate::app::commands::doctor::schemas::dates::ensure_date;
use crate::app::commands::doctor::yaml::{
    ensure_int, ensure_non_empty_sequence, load_yaml_mapping,
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
    ensure_date(data, path, "created_at", diagnostics);

    // Validate summaries sequence
    ensure_non_empty_sequence(data, path, "summaries", diagnostics);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::commands::doctor::diagnostics::Diagnostics;
    use std::path::PathBuf;

    #[test]
    fn validates_changes_created_at_date_format() {
        let yaml = r#"
schema_version: 1
created_at: "2026-02-19"
summaries:
  - title: "a"
    scope: "b"
    impact: "c"
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let mut diagnostics = Diagnostics::default();
        validate_changes_data(&data, &PathBuf::from("changes.yml"), &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn rejects_changes_created_at_datetime_format() {
        let yaml = r#"
schema_version: 1
created_at: "2026-02-19T00:00:00Z"
summaries:
  - title: "a"
    scope: "b"
    impact: "c"
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let mut diagnostics = Diagnostics::default();
        validate_changes_data(&data, &PathBuf::from("changes.yml"), &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 1);
        assert!(diagnostics.errors()[0].message.contains("created_at must be YYYY-MM-DD"));
    }
}
