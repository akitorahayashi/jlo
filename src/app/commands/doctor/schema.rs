use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde_yaml::Mapping;

use crate::domain::{AppError, Layer};

use super::diagnostics::Diagnostics;
use super::structure::list_subdirs;
use super::yaml::{
    ensure_enum, ensure_id, ensure_int, ensure_non_empty_sequence, ensure_non_empty_string,
    get_bool, get_sequence, get_sequence_strings, get_string, load_yaml_mapping, read_yaml_files,
};

#[derive(Debug, Clone)]
pub(crate) struct PromptEntry {
    pub path: PathBuf,
    pub contracts: Vec<String>,
}

pub struct SchemaInputs<'a> {
    pub jules_path: &'a Path,
    pub root: &'a Path,
    pub workstreams: &'a [String],
    pub issue_labels: &'a [String],
    pub event_states: &'a [String],
    pub event_confidence: &'a [String],
    pub issue_priorities: &'a [String],
    pub prompt_entries: &'a [PromptEntry],
}

pub fn collect_prompt_entries(
    jules_path: &Path,
    diagnostics: &mut Diagnostics,
) -> Result<Vec<PromptEntry>, AppError> {
    let mut entries = Vec::new();

    for layer in Layer::ALL {
        let layer_dir = jules_path.join("roles").join(layer.dir_name());
        if !layer_dir.exists() {
            continue;
        }

        if layer.is_single_role() {
            // Single-role layers have prompt.yml directly in layer directory
            let prompt_path = layer_dir.join("prompt.yml");
            if prompt_path.exists()
                && let Some(entry) = parse_prompt(&prompt_path, layer, diagnostics)
            {
                entries.push(entry);
            }
        } else {
            // Multi-role layers have role.yml in each role subdirectory under roles/
            let roles_container = layer_dir.join("roles");
            if roles_container.exists() {
                for role_dir in list_subdirs(&roles_container, diagnostics) {
                    let role_path = role_dir.join("role.yml");
                    if role_path.exists()
                        && let Some(entry) = parse_role_file(&role_path, layer, diagnostics)
                    {
                        entries.push(entry);
                    }
                }
            }
        }
    }

    Ok(entries)
}

pub fn schema_checks(inputs: SchemaInputs<'_>, diagnostics: &mut Diagnostics) {
    for entry in inputs.prompt_entries {
        for contract in &entry.contracts {
            let contract_path = inputs.root.join(contract);
            if !contract_path.exists() {
                diagnostics.push_error(
                    entry.path.display().to_string(),
                    format!("Contract not found: {}", contract),
                );
            }
        }
    }

    // Validate changes/latest.yml if present
    let latest_path = inputs.jules_path.join("changes").join("latest.yml");
    if latest_path.exists() {
        let change_schema_path =
            inputs.jules_path.join("roles").join("narrator").join("schemas").join("change.yml");
        validate_changes_latest(&latest_path, &change_schema_path, diagnostics);
    }

    for layer in Layer::ALL {
        let layer_dir = inputs.jules_path.join("roles").join(layer.dir_name());
        if !layer_dir.exists() {
            continue;
        }

        let contracts_path = layer_dir.join("contracts.yml");
        if contracts_path.exists() {
            validate_contracts_file(&contracts_path, layer, diagnostics);
        }

        if layer == Layer::Observers || layer == Layer::Deciders {
            let roles_container = layer_dir.join("roles");
            if roles_container.exists() {
                for role_dir in list_subdirs(&roles_container, diagnostics) {
                    let role_path = role_dir.join("role.yml");
                    if role_path.exists() {
                        if layer == Layer::Observers {
                            validate_role_file(&role_path, &role_dir, diagnostics);
                        } else {
                            validate_decider_role_file(&role_path, diagnostics);
                        }
                    }
                }
            }
        }
    }

    for workstream in inputs.workstreams {
        let ws_dir = inputs.jules_path.join("workstreams").join(workstream);
        let exchange_dir = ws_dir.join("exchange");

        let events_dir = exchange_dir.join("events");
        for state in inputs.event_states {
            let state_dir = events_dir.join(state);
            for entry in read_yaml_files(&state_dir, diagnostics) {
                validate_event_file(&entry, state, inputs.event_confidence, diagnostics);
                check_placeholders_file(&entry, diagnostics);
            }
        }

        let issues_dir = exchange_dir.join("issues");
        for label in inputs.issue_labels {
            let label_dir = issues_dir.join(label);
            for entry in read_yaml_files(&label_dir, diagnostics) {
                validate_issue_file(
                    &entry,
                    label,
                    inputs.issue_labels,
                    inputs.issue_priorities,
                    diagnostics,
                );
                check_placeholders_file(&entry, diagnostics);
            }
        }
    }
}

fn parse_prompt(path: &Path, layer: Layer, diagnostics: &mut Diagnostics) -> Option<PromptEntry> {
    let data = load_yaml_mapping(path, diagnostics)?;
    parse_prompt_data(&data, path, layer, diagnostics)
}

fn parse_prompt_data(
    data: &Mapping,
    path: &Path,
    layer: Layer,
    diagnostics: &mut Diagnostics,
) -> Option<PromptEntry> {
    let role = get_string(data, "role");
    if role.as_deref().unwrap_or("").is_empty() {
        diagnostics.push_error(path.display().to_string(), "Missing role field");
    }

    let layer_field = get_string(data, "layer").unwrap_or_default();
    if layer_field != layer.dir_name() {
        diagnostics.push_error(
            path.display().to_string(),
            format!("Layer field '{}' does not match {}", layer_field, layer.dir_name()),
        );
    }

    let contracts = get_sequence_strings(data, "contracts");
    if contracts.is_empty() {
        diagnostics.push_error(path.display().to_string(), "Missing contracts list");
    }

    let instructions = get_sequence_strings(data, "instructions");
    if instructions.is_empty() {
        diagnostics.push_error(path.display().to_string(), "Missing instructions list");
    }

    // Single-role layers should not have workstream field
    if layer.is_single_role() {
        let workstream = get_string(data, "workstream");
        if workstream.is_some() {
            diagnostics.push_error(
                path.display().to_string(),
                "workstream not allowed in single-role layer",
            );
        }
    }

    Some(PromptEntry { path: path.to_path_buf(), contracts })
}

fn validate_event_file(
    path: &Path,
    state: &str,
    event_confidence: &[String],
    diagnostics: &mut Diagnostics,
) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };
    validate_event(&data, path, state, event_confidence, diagnostics);
}

pub fn validate_event(
    data: &Mapping,
    path: &Path,
    state: &str,
    event_confidence: &[String],
    diagnostics: &mut Diagnostics,
) {
    ensure_int(data, path, "schema_version", diagnostics, Some(1));
    ensure_id(data, path, "id", diagnostics);

    let issue_id = match get_string(data, "issue_id") {
        Some(value) => value,
        None => {
            diagnostics.push_error(path.display().to_string(), "issue_id is required");
            String::new()
        }
    };
    if state == "pending" && !issue_id.is_empty() {
        diagnostics.push_error(path.display().to_string(), "issue_id must be empty in pending");
    }
    if state == "decided" && issue_id.is_empty() {
        diagnostics.push_error(path.display().to_string(), "issue_id must be set in decided");
    }

    ensure_date(data, path, "created_at", diagnostics);
    ensure_non_empty_string(data, path, "author_role", diagnostics);
    let allowed: Vec<&str> = event_confidence.iter().map(|value| value.as_str()).collect();
    ensure_enum(data, path, "confidence", &allowed, diagnostics);
    ensure_non_empty_string(data, path, "title", diagnostics);
    ensure_non_empty_string(data, path, "statement", diagnostics);

    if let Some(evidence) = get_sequence(data, "evidence") {
        if evidence.is_empty() {
            diagnostics.push_error(path.display().to_string(), "evidence must have entries");
        } else {
            for (idx, entry) in evidence.iter().enumerate() {
                if let serde_yaml::Value::Mapping(map) = entry {
                    if get_string(map, "path").unwrap_or_default().is_empty() {
                        diagnostics.push_error(
                            path.display().to_string(),
                            format!("evidence[{}].path is required", idx),
                        );
                    }
                    if get_sequence(map, "loc").map(|seq| seq.is_empty()).unwrap_or(true) {
                        diagnostics.push_error(
                            path.display().to_string(),
                            format!("evidence[{}].loc is required", idx),
                        );
                    }
                    if get_string(map, "note").unwrap_or_default().is_empty() {
                        diagnostics.push_error(
                            path.display().to_string(),
                            format!("evidence[{}].note is required", idx),
                        );
                    }
                } else {
                    diagnostics.push_error(
                        path.display().to_string(),
                        format!("evidence[{}] must be a map", idx),
                    );
                }
            }
        }
    } else {
        diagnostics.push_error(path.display().to_string(), "Missing evidence list");
    }
}

fn validate_issue_file(
    path: &Path,
    label: &str,
    issue_labels: &[String],
    issue_priorities: &[String],
    diagnostics: &mut Diagnostics,
) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };
    validate_issue(&data, path, label, issue_labels, issue_priorities, diagnostics);
}

pub fn validate_issue(
    data: &Mapping,
    path: &Path,
    label: &str,
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
                && !super::yaml::is_valid_id(&value)
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
    if label_value != label {
        diagnostics.push_error(
            path.display().to_string(),
            format!("label '{}' does not match directory '{}'", label_value, label),
        );
    }
    if !label_value.is_empty() && !issue_labels.contains(&label_value) {
        diagnostics.push_error(path.display().to_string(), "label is not recognized");
    }

    let allowed: Vec<&str> = issue_priorities.iter().map(|value| value.as_str()).collect();
    ensure_enum(data, path, "priority", &allowed, diagnostics);

    ensure_non_empty_string(data, path, "summary", diagnostics);
    ensure_non_empty_string(data, path, "problem", diagnostics);
    ensure_non_empty_string(data, path, "impact", diagnostics);
    ensure_non_empty_string(data, path, "desired_outcome", diagnostics);

    if get_sequence(data, "affected_areas").map(|seq| seq.is_empty()).unwrap_or(true) {
        diagnostics.push_error(path.display().to_string(), "affected_areas must have entries");
    }

    if get_sequence(data, "acceptance_criteria").map(|seq| seq.is_empty()).unwrap_or(true) {
        diagnostics.push_error(path.display().to_string(), "acceptance_criteria must have entries");
    }

    if get_sequence(data, "verification_commands").map(|seq| seq.is_empty()).unwrap_or(true) {
        diagnostics
            .push_error(path.display().to_string(), "verification_commands must have entries");
    } else if let Some(seq) = get_sequence(data, "verification_commands") {
        for command in seq {
            if let serde_yaml::Value::String(value) = command {
                let value_lower = value.to_lowercase();
                if value_lower.contains("jules") || value_lower.contains("jlo run") {
                    diagnostics.push_error(
                        path.display().to_string(),
                        "verification_commands must not invoke jules or jlo run",
                    );
                    break;
                }
            }
        }
    }

    let requires_deep = match get_bool(data, "requires_deep_analysis") {
        Some(val) => val,
        None => {
            diagnostics.push_error(
                path.display().to_string(),
                "requires_deep_analysis is required (true/false)",
            );
            return;
        }
    };

    let deep_reason = get_string(data, "deep_analysis_reason").unwrap_or_default();
    if requires_deep && deep_reason.trim().is_empty() {
        diagnostics.push_error(
            path.display().to_string(),
            "deep_analysis_reason required when requires_deep_analysis is true",
        );
    }
}

fn validate_role_file(path: &Path, role_dir: &Path, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };
    validate_role(&data, path, role_dir, diagnostics);
}

pub fn validate_role(data: &Mapping, path: &Path, role_dir: &Path, diagnostics: &mut Diagnostics) {
    ensure_non_empty_string(data, path, "role", diagnostics);

    // Check layer field
    let layer_value = get_string(data, "layer").unwrap_or_default();
    if layer_value != "observers" {
        diagnostics.push_error(path.display().to_string(), "layer must be 'observers'");
    }

    // Check profile section
    match data.get("profile") {
        Some(serde_yaml::Value::Mapping(profile_map)) => {
            if get_string(profile_map, "focus").is_none() {
                diagnostics.push_error(path.display().to_string(), "Missing profile.focus");
            }
            if get_sequence(profile_map, "analysis_points")
                .map(|seq| seq.is_empty())
                .unwrap_or(true)
            {
                diagnostics.push_error(
                    path.display().to_string(),
                    "profile.analysis_points must have entries",
                );
            }
        }
        Some(_) => {
            diagnostics.push_error(path.display().to_string(), "'profile' must be a mapping");
        }
        None => {
            diagnostics.push_error(path.display().to_string(), "Missing profile section");
        }
    }

    let role_name = role_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let role_value = get_string(data, "role").unwrap_or_default();
    if !role_value.is_empty() && role_value != role_name {
        diagnostics.push_error(
            path.display().to_string(),
            format!("role '{}' does not match directory '{}'", role_value, role_name),
        );
    }
}

/// Parse multi-role layer role.yml for entry collection
fn parse_role_file(
    path: &Path,
    layer: Layer,
    diagnostics: &mut Diagnostics,
) -> Option<PromptEntry> {
    let data = load_yaml_mapping(path, diagnostics)?;
    parse_role_file_data(&data, path, layer, diagnostics)
}

fn parse_role_file_data(
    data: &Mapping,
    path: &Path,
    layer: Layer,
    diagnostics: &mut Diagnostics,
) -> Option<PromptEntry> {
    let role = get_string(data, "role").unwrap_or_default();
    if role.is_empty() {
        diagnostics.push_error(path.display().to_string(), "Missing role field");
    }

    let layer_field = get_string(data, "layer").unwrap_or_default();
    if layer_field != layer.dir_name() {
        diagnostics.push_error(
            path.display().to_string(),
            format!("Layer field '{}' does not match {}", layer_field, layer.dir_name()),
        );
    }

    // Multi-role layers don't have contracts in role.yml (handled by prompt_assembly.yml)
    Some(PromptEntry { path: path.to_path_buf(), contracts: vec![] })
}

/// Validate decider role.yml schema
fn validate_decider_role_file(path: &Path, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };
    validate_decider_role(&data, path, diagnostics);
}

pub fn validate_decider_role(data: &Mapping, path: &Path, diagnostics: &mut Diagnostics) {
    ensure_non_empty_string(data, path, "role", diagnostics);

    let layer_value = get_string(data, "layer").unwrap_or_default();
    if layer_value != "deciders" {
        diagnostics.push_error(path.display().to_string(), "layer must be 'deciders'");
    }

    // Check profile section
    match data.get("profile") {
        Some(profile) if !profile.is_mapping() => {
            diagnostics.push_error(path.display().to_string(), "'profile' must be a mapping");
        }
        None => {
            diagnostics.push_error(path.display().to_string(), "Missing profile section");
        }
        _ => {}
    }
}

fn validate_contracts_file(path: &Path, layer: Layer, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };
    validate_contracts(&data, path, layer, diagnostics);
}

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

    ensure_non_empty_sequence(data, path, "constraints", diagnostics);
    ensure_non_empty_sequence(data, path, "inputs", diagnostics);
    ensure_non_empty_sequence(data, path, "outputs", diagnostics);
    ensure_non_empty_sequence(data, path, "workflow", diagnostics);
}

/// Validate .jules/changes/latest.yml schema.
fn validate_changes_latest(path: &Path, template_path: &Path, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };

    let allowed_modes = extract_enum_from_template(template_path, "selection_mode");
    validate_changes_latest_data(&data, path, &allowed_modes, diagnostics);
}

fn validate_changes_latest_data(
    data: &Mapping,
    path: &Path,
    allowed_modes: &[String],
    diagnostics: &mut Diagnostics,
) {
    // Required fields
    ensure_int(data, path, "schema_version", diagnostics, Some(1));
    ensure_id(data, path, "id", diagnostics);
    ensure_non_empty_string(data, path, "created_at", diagnostics);

    // Validate range mapping
    if let Some(range) = data.get(serde_yaml::Value::String("range".to_string())) {
        if let serde_yaml::Value::Mapping(range_map) = range {
            ensure_non_empty_string(range_map, path, "from_commit", diagnostics);
            ensure_non_empty_string(range_map, path, "to_commit", diagnostics);

            // Validate selection_mode enum from template
            if !allowed_modes.is_empty() {
                let allowed_refs: Vec<&str> = allowed_modes.iter().map(|s| s.as_str()).collect();
                ensure_enum(range_map, path, "selection_mode", &allowed_refs, diagnostics);
            }

            // Validate .jules/ is in excluded_paths
            if let Some(excluded) = get_sequence(range_map, "excluded_paths") {
                let has_jules = excluded.iter().any(|v| {
                    if let serde_yaml::Value::String(s) = v {
                        s == ".jules/" || s == ".jules"
                    } else {
                        false
                    }
                });
                if !has_jules {
                    diagnostics.push_error(
                        path.display().to_string(),
                        "range.excluded_paths must include .jules/",
                    );
                }
            } else {
                diagnostics.push_error(path.display().to_string(), "Missing range.excluded_paths");
            }
        } else {
            diagnostics.push_error(path.display().to_string(), "range must be a mapping");
        }
    } else {
        diagnostics.push_error(path.display().to_string(), "Missing range field");
    }

    // Validate stats mapping
    if let Some(stats) = data.get(serde_yaml::Value::String("stats".to_string())) {
        if let serde_yaml::Value::Mapping(stats_map) = stats {
            ensure_int(stats_map, path, "commits_total", diagnostics, None);
            ensure_int(stats_map, path, "commits_included", diagnostics, None);
            ensure_int(stats_map, path, "files_changed", diagnostics, None);
            ensure_int(stats_map, path, "insertions", diagnostics, None);
            ensure_int(stats_map, path, "deletions", diagnostics, None);
        } else {
            diagnostics.push_error(path.display().to_string(), "stats must be a mapping");
        }
    } else {
        diagnostics.push_error(path.display().to_string(), "Missing stats field");
    }

    // Validate commits list exists
    if get_sequence(data, "commits").is_none() {
        diagnostics.push_error(path.display().to_string(), "Missing commits list");
    }

    // Validate summary mapping
    if let Some(summary) = data.get(serde_yaml::Value::String("summary".to_string())) {
        if let serde_yaml::Value::Mapping(summary_map) = summary {
            ensure_non_empty_string(summary_map, path, "overview", diagnostics);
        } else {
            diagnostics.push_error(path.display().to_string(), "summary must be a mapping");
        }
    } else {
        diagnostics.push_error(path.display().to_string(), "Missing summary field");
    }
}

/// Extract allowed enum values from the change.yml template by parsing comments.
/// Looks for lines like: `selection_mode: value  # Allowed: value1, value2`
fn extract_enum_from_template(template_path: &Path, field: &str) -> Vec<String> {
    let content = match fs::read_to_string(template_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    for line in content.lines() {
        let trimmed = line.trim();
        // Look for the field name at the start of the line
        if trimmed.starts_with(&format!("{}:", field)) {
            // Look for "# Allowed:" comment in the same line
            if let Some(comment_start) = line.find("# Allowed:") {
                let allowed_part = &line[comment_start + "# Allowed:".len()..];
                return allowed_part.split(',').map(|s| s.trim().to_string()).collect();
            }
        }
    }

    vec![]
}

fn check_placeholders_file(path: &Path, diagnostics: &mut Diagnostics) {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            diagnostics
                .push_error(path.display().to_string(), format!("Failed to read file: {}", err));
            return;
        }
    };
    check_placeholders(&content, path, diagnostics);
}

pub fn check_placeholders(content: &str, path: &Path, diagnostics: &mut Diagnostics) {
    let placeholders = [
        "<6_random_lowercase_alphanumeric_chars>",
        "<role>",
        "<workstream>",
        "<Descriptive Title>",
        "YYYY-MM-DD",
        "<path>",
        "<condition 1>",
        "<condition 2>",
    ];

    for placeholder in placeholders {
        if content.contains(placeholder) {
            diagnostics.push_error(
                path.display().to_string(),
                format!("placeholder '{}' must be replaced", placeholder),
            );
        }
    }
}

fn ensure_date(map: &serde_yaml::Mapping, path: &Path, key: &str, diagnostics: &mut Diagnostics) {
    let value = get_string(map, key).unwrap_or_default();
    if NaiveDate::parse_from_str(&value, "%Y-%m-%d").is_err() {
        diagnostics.push_error(path.display().to_string(), format!("{} must be YYYY-MM-DD", key));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_event_data_valid() {
        let yaml = r#"
schema_version: 1
id: "abc123"
issue_id: ""
created_at: "2023-10-27"
author_role: "observer"
confidence: "high"
title: "Something happened"
statement: "Evidence suggests..."
evidence:
  - path: "src/main.rs"
    loc: ["10-20"]
    note: "See this"
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("test.yml");
        let mut diagnostics = Diagnostics::default();
        let confidence = vec!["high".to_string(), "low".to_string()];

        validate_event(&data, &path, "pending", &confidence, &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn test_validate_event_data_invalid_state() {
        let yaml = r#"
schema_version: 1
id: "abc123"
issue_id: "xyz789"  # Should be empty for pending
created_at: "2023-10-27"
author_role: "observer"
confidence: "high"
title: "Something happened"
statement: "Evidence suggests..."
evidence: []
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("test.yml");
        let mut diagnostics = Diagnostics::default();
        let confidence = vec!["high".to_string()];

        validate_event(&data, &path, "pending", &confidence, &mut diagnostics);
        assert!(diagnostics.error_count() > 0);
        // Should have errors: issue_id must be empty in pending, evidence must have entries
    }

    #[test]
    fn test_check_placeholders_content() {
        let content = "This is a <role> description.";
        let path = PathBuf::from("test.yml");
        let mut diagnostics = Diagnostics::default();

        check_placeholders(content, &path, &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 1);
        assert!(diagnostics.errors()[0].message.contains("placeholder '<role>' must be replaced"));
    }

    #[test]
    fn test_validate_issue_data_valid() {
        let yaml = r#"
schema_version: 2
requires_deep_analysis: false
id: "abc123"
source_events: ["ev1234"]
title: "Bug fix"
label: "bugs"
priority: "high"
summary: "Summary"
problem: "Problem"
impact: "Impact"
desired_outcome: "Outcome"
affected_areas: ["src/"]
acceptance_criteria: ["Done"]
verification_commands: ["cargo test"]
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("test.yml");
        let mut diagnostics = Diagnostics::default();
        let labels = vec!["bugs".to_string()];
        let priorities = vec!["high".to_string()];

        validate_issue(&data, &path, "bugs", &labels, &priorities, &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn test_validate_issue_missing_requires_deep_analysis() {
        let yaml = r#"
schema_version: 2
id: "abc123"
source_events: ["ev1234"]
title: "Bug fix"
label: "bugs"
priority: "high"
summary: "Summary"
problem: "Problem"
impact: "Impact"
desired_outcome: "Outcome"
affected_areas: ["src/"]
acceptance_criteria: ["Done"]
verification_commands: ["cargo test"]
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("test.yml");
        let mut diagnostics = Diagnostics::default();
        let labels = vec!["bugs".to_string()];
        let priorities = vec!["high".to_string()];

        validate_issue(&data, &path, "bugs", &labels, &priorities, &mut diagnostics);
        assert!(diagnostics.error_count() > 0);
        assert!(diagnostics.errors()[0].message.contains("requires_deep_analysis is required"));
    }
}
