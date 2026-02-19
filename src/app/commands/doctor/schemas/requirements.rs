use serde_yaml::Mapping;
use std::path::Path;

use crate::app::commands::doctor::diagnostics::Diagnostics;
use crate::app::commands::doctor::yaml::{
    ensure_enum, ensure_id, ensure_int, ensure_non_empty_string, get_bool, get_sequence,
    get_string, load_yaml_mapping,
};

pub fn validate_requirement_file(
    path: &Path,
    issue_labels: &[String],
    issue_priorities: &[String],
    diagnostics: &mut Diagnostics,
) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };
    validate_requirement(&data, path, issue_labels, issue_priorities, diagnostics);
}

pub fn validate_requirement(
    data: &Mapping,
    path: &Path,
    issue_labels: &[String],
    issue_priorities: &[String],
    diagnostics: &mut Diagnostics,
) {
    ensure_int(data, path, "schema_version", diagnostics, Some(2));
    ensure_id(data, path, "id", diagnostics);
    if get_sequence(data, "source_events").map(|seq| seq.is_empty()).unwrap_or(true) {
        diagnostics.push_error(path.display().to_string(), "source_events must have entries");
    } else if let Some(seq) = get_sequence(data, "source_events") {
        for event_id in seq {
            if let serde_yaml::Value::String(value) = event_id
                && !crate::app::commands::doctor::yaml::is_valid_id(&value)
            {
                diagnostics.push_error(
                    path.display().to_string(),
                    format!("Invalid source_events id: {}", value),
                );
            }
        }
    }

    ensure_non_empty_string(data, path, "title", diagnostics);
    ensure_non_empty_string(data, path, "label", diagnostics);

    let label_value = get_string(data, "label").unwrap_or_default();
    if !label_value.is_empty() && !issue_labels.contains(&label_value) {
        diagnostics.push_error(
            path.display().to_string(),
            format!("label '{}' is not defined in github-labels.json", label_value),
        );
    }

    let allowed: Vec<&str> = issue_priorities.iter().map(|value| value.as_str()).collect();
    ensure_enum(data, path, "priority", &allowed, diagnostics);

    ensure_non_empty_string(data, path, "summary", diagnostics);
    ensure_non_empty_string(data, path, "goal", diagnostics);
    ensure_non_empty_string(data, path, "problem", diagnostics);
    ensure_non_empty_string(data, path, "impact", diagnostics);
    ensure_non_empty_string(data, path, "desired_outcome", diagnostics);

    if get_sequence(data, "affected_areas").map(|seq| seq.is_empty()).unwrap_or(true) {
        diagnostics.push_error(path.display().to_string(), "affected_areas must have entries");
    }

    if get_sequence(data, "acceptance_criteria").map(|seq| seq.is_empty()).unwrap_or(true) {
        diagnostics.push_error(path.display().to_string(), "acceptance_criteria must have entries");
    }

    if get_sequence(data, "verification_criteria").map(|seq| seq.is_empty()).unwrap_or(true) {
        diagnostics
            .push_error(path.display().to_string(), "verification_criteria must have entries");
    }

    let implementation_ready = match get_bool(data, "implementation_ready") {
        Some(val) => val,
        None => {
            diagnostics.push_error(
                path.display().to_string(),
                "implementation_ready is required (true/false)",
            );
            return;
        }
    };

    let planner_reason = get_string(data, "planner_request_reason").unwrap_or_default();
    if !implementation_ready && planner_reason.trim().is_empty() {
        diagnostics.push_error(
            path.display().to_string(),
            "planner_request_reason required when implementation_ready is false",
        );
    }

    if implementation_ready && !planner_reason.trim().is_empty() {
        diagnostics.push_error(
            path.display().to_string(),
            "planner_request_reason must be empty when implementation_ready is true",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_validate_requirement_data_valid() {
        let yaml = r#"
schema_version: 2
implementation_ready: true
planner_request_reason: ""
id: "abc123"
source_events: ["ev1234"]
title: "Bug fix"
label: "bugs"
priority: "high"
summary: "Summary"
goal: "Goal"
problem: "Problem"
impact: "Impact"
desired_outcome: "Outcome"
affected_areas: ["src/"]
acceptance_criteria: ["Done"]
verification_criteria: ["test commands"]
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("test.yml");
        let mut diagnostics = Diagnostics::default();
        let labels = vec!["bugs".to_string()];
        let priorities = vec!["high".to_string()];

        validate_requirement(&data, &path, &labels, &priorities, &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn test_validate_requirement_missing_implementation_ready() {
        let yaml = r#"
schema_version: 2
id: "abc123"
source_events: ["ev1234"]
title: "Bug fix"
label: "bugs"
priority: "high"
summary: "Summary"
goal: "Goal"
problem: "Problem"
impact: "Impact"
desired_outcome: "Outcome"
affected_areas: ["src/"]
acceptance_criteria: ["Done"]
verification_criteria: ["test commands"]
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("test.yml");
        let mut diagnostics = Diagnostics::default();
        let labels = vec!["bugs".to_string()];
        let priorities = vec!["high".to_string()];

        validate_requirement(&data, &path, &labels, &priorities, &mut diagnostics);
        assert!(diagnostics.error_count() > 0);
        assert!(diagnostics.errors()[0].message.contains("implementation_ready is required"));
    }
}
