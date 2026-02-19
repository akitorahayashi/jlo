use serde_yaml::Mapping;
use std::path::Path;

use crate::app::commands::doctor::diagnostics::Diagnostics;
use crate::app::commands::doctor::yaml::{
    ensure_id, ensure_int, ensure_non_empty_sequence, ensure_non_empty_string, get_string,
    load_yaml_mapping,
};

use super::dates::ensure_date;

pub fn validate_innovator_proposal(path: &Path, diagnostics: &mut Diagnostics) {
    if let Some(data) = load_yaml_mapping(path, diagnostics) {
        validate_innovator_document_dates_fields(&data, path, diagnostics);
        ensure_non_empty_string(&data, path, "introduction", diagnostics);
        ensure_non_empty_string(&data, path, "importance", diagnostics);
        ensure_non_empty_sequence(&data, path, "impact_surface", diagnostics);
        ensure_non_empty_string(&data, path, "implementation_cost", diagnostics);
        ensure_non_empty_sequence(&data, path, "consistency_risks", diagnostics);
        ensure_non_empty_sequence(&data, path, "verification_signals", diagnostics);

        let role = get_string(&data, "role").unwrap_or_default();
        if !role.is_empty()
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            let expected_role_segment =
                crate::domain::exchange::proposals::paths::proposal_filename_role_segment(&role);
            if !stem.starts_with(&format!("{}-", expected_role_segment)) {
                diagnostics.push_error(
                    path.display().to_string(),
                    format!(
                        "proposal filename must start with normalized role '{}-'",
                        expected_role_segment
                    ),
                );
            }
        }
    }
}

pub fn validate_innovator_document_dates_fields(
    data: &Mapping,
    path: &Path,
    diagnostics: &mut Diagnostics,
) {
    ensure_int(data, path, "schema_version", diagnostics, Some(1));
    ensure_id(data, path, "id", diagnostics);
    ensure_non_empty_string(data, path, "role", diagnostics);
    ensure_date(data, path, "created_at", diagnostics);
    ensure_non_empty_string(data, path, "title", diagnostics);
    ensure_non_empty_string(data, path, "problem", diagnostics);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_validate_innovator_proposal_accepts_normalized_role_prefix() {
        let dir = tempdir().expect("tempdir");
        let proposal_path = dir.path().join("leverage-architect-mock-proposal-1.yml");
        fs::write(
            &proposal_path,
            r#"
schema_version: 1
id: "abc123"
role: "leverage_architect"
created_at: "2026-02-17"
title: "Mock proposal"
problem: "p"
introduction: "i"
importance: "m"
impact_surface: ["a"]
implementation_cost: "c"
consistency_risks: ["r"]
verification_signals: ["v"]
"#,
        )
        .expect("write proposal");

        let mut diagnostics = Diagnostics::default();
        validate_innovator_proposal(&proposal_path, &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);
    }
}
