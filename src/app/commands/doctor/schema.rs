use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde_yaml::Mapping;

use crate::domain::{AppError, Layer};

use super::diagnostics::Diagnostics;
use super::structure::list_subdirs;
use super::yaml::{
    ensure_enum, ensure_id, ensure_int, ensure_non_empty_sequence, ensure_non_empty_string,
    get_bool, get_sequence, get_string, load_yaml_mapping, read_yaml_files,
};

const DATETIME_PLACEHOLDER: &str = "YYYY-MM-DD";

#[derive(Debug, Clone)]
pub(crate) struct PromptEntry {
    pub path: PathBuf,
    pub contracts: Vec<String>,
}

pub struct SchemaInputs<'a> {
    pub jules_path: &'a Path,
    pub root: &'a Path,
    pub issue_labels: &'a [String],
    pub event_states: &'a [String],
    pub event_confidence: &'a [String],
    pub issue_priorities: &'a [String],
    pub prompt_entries: &'a [PromptEntry],
}

pub fn collect_prompt_entries(
    _jules_path: &Path,
    _diagnostics: &mut Diagnostics,
) -> Result<Vec<PromptEntry>, AppError> {
    // Prompt entries (templates, contracts, tasks) are now embedded in the binary
    // via src/assets/prompt-assemble/. No filesystem-based collection needed.
    Ok(Vec::new())
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

    let changes_path = crate::domain::exchange::paths::exchange_changes(inputs.jules_path);
    if changes_path.exists() {
        validate_exchange_changes(&changes_path, diagnostics);
    }

    // Validate embedded contracts for each layer
    for layer in Layer::ALL {
        let catalog_path = format!("{}/contracts.yml", layer.dir_name());
        if let Some(content) =
            crate::adapters::catalogs::prompt_assemble_assets::read_prompt_assemble_asset(
                &catalog_path,
            )
            && let Ok(data) = serde_yaml::from_str::<serde_yaml::Value>(&content)
            && let Some(mapping) = data.as_mapping()
        {
            let label = format!("prompt-assemble://{}", catalog_path);
            validate_contracts(mapping, Path::new(&label), layer, diagnostics);
        }

        // Validate role definitions in .jlo/roles/ for multi-role layers
        if layer == Layer::Observers {
            let jlo_layer_dir = crate::domain::roles::paths::layer_dir(inputs.root, layer);
            if jlo_layer_dir.exists() {
                for role_dir in list_subdirs(&jlo_layer_dir, diagnostics) {
                    let role_path = role_dir.join("role.yml");
                    if role_path.exists() {
                        validate_role_file(&role_path, &role_dir, diagnostics);
                    }
                }
            }
        }

        if layer == Layer::Innovators {
            let jlo_layer_dir = crate::domain::roles::paths::layer_dir(inputs.root, layer);
            if jlo_layer_dir.exists() {
                for role_dir in list_subdirs(&jlo_layer_dir, diagnostics) {
                    let role_path = role_dir.join("role.yml");
                    if role_path.exists() {
                        validate_innovator_role_file(&role_path, &role_dir, diagnostics);
                    }
                }
            }
        }
    }

    // Validate flat exchange directory
    {
        for state in inputs.event_states {
            let state_dir =
                crate::domain::exchange::events::paths::events_state_dir(inputs.jules_path, state);
            for entry in read_yaml_files(&state_dir, diagnostics) {
                validate_event_file(&entry, state, inputs.event_confidence, diagnostics);
                check_placeholders_file(&entry, diagnostics);
            }
        }

        {
            let requirements_dir =
                crate::domain::exchange::requirements::paths::requirements_dir(inputs.jules_path);
            for entry in read_yaml_files(&requirements_dir, diagnostics) {
                validate_requirement_file(
                    &entry,
                    inputs.issue_labels,
                    inputs.issue_priorities,
                    diagnostics,
                );
                check_placeholders_file(&entry, diagnostics);
            }
        }

        for role_name in scheduled_innovator_roles(inputs.root, diagnostics) {
            let perspective_path = crate::domain::workstations::paths::workstation_perspective(
                inputs.jules_path,
                &role_name,
            );
            if !perspective_path.exists() {
                diagnostics.push_error(
                    perspective_path.display().to_string(),
                    "Missing innovator workstation perspective.yml",
                );
                continue;
            }
            validate_innovator_perspective(&perspective_path, &role_name, diagnostics);
        }

        for role_name in scheduled_observer_roles(inputs.root, diagnostics) {
            let perspective_path = crate::domain::workstations::paths::workstation_perspective(
                inputs.jules_path,
                &role_name,
            );
            if !perspective_path.exists() {
                diagnostics.push_error(
                    perspective_path.display().to_string(),
                    "Missing observer workstation perspective.yml",
                );
                continue;
            }
            validate_observer_perspective(&perspective_path, &role_name, diagnostics);
        }

        let proposals_dir =
            crate::domain::exchange::proposals::paths::proposals_dir(inputs.jules_path);
        for proposal_path in read_yaml_files(&proposals_dir, diagnostics) {
            validate_innovator_proposal(&proposal_path, diagnostics);
            check_placeholders_file(&proposal_path, diagnostics);
        }
    }
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

    let requirement_id = match get_string(data, "requirement_id") {
        Some(value) => value,
        None => {
            diagnostics.push_error(path.display().to_string(), "requirement_id is required");
            String::new()
        }
    };
    if state == "pending" && !requirement_id.is_empty() {
        diagnostics
            .push_error(path.display().to_string(), "requirement_id must be empty in pending");
    }
    if state == "decided" && requirement_id.is_empty() {
        diagnostics.push_error(path.display().to_string(), "requirement_id must be set in decided");
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

fn validate_requirement_file(
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

    // constraints is optional — some layers may not need explicit guardrails
}

/// Validate .jules/exchange/changes.yml schema.
fn validate_exchange_changes(path: &Path, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };

    validate_exchange_changes_data(&data, path, diagnostics);
}

fn validate_exchange_changes_data(data: &Mapping, path: &Path, diagnostics: &mut Diagnostics) {
    // Required fields
    ensure_int(data, path, "schema_version", diagnostics, Some(1));
    ensure_non_empty_string(data, path, "created_at", diagnostics);

    // Validate summaries sequence
    ensure_non_empty_sequence(data, path, "summaries", diagnostics);
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
        "<Descriptive Title>",
        DATETIME_PLACEHOLDER,
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

fn ensure_datetime(
    map: &serde_yaml::Mapping,
    path: &Path,
    key: &str,
    diagnostics: &mut Diagnostics,
) {
    let value = get_string(map, key).unwrap_or_default();
    if NaiveDate::parse_from_str(&value, "%Y-%m-%d").is_err() {
        diagnostics.push_error(
            path.display().to_string(),
            format!("{} must be YYYY-MM-DD ({})", key, DATETIME_PLACEHOLDER),
        );
    }
}

fn get_scheduled_roles<F>(
    root: &Path,
    diagnostics: &mut Diagnostics,
    role_extractor: F,
) -> Vec<String>
where
    F: FnOnce(crate::domain::ControlPlaneConfig) -> Vec<String>,
{
    let config_path = crate::domain::config::paths::config(root);
    let content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(err) => {
            diagnostics.push_error(config_path.display().to_string(), err.to_string());
            return Vec::new();
        }
    };

    let config = match crate::domain::config::parse::parse_config_content(&content) {
        Ok(config) => config,
        Err(err) => {
            diagnostics.push_error(config_path.display().to_string(), err.to_string());
            return Vec::new();
        }
    };

    role_extractor(config)
}

fn scheduled_innovator_roles(root: &Path, diagnostics: &mut Diagnostics) -> Vec<String> {
    get_scheduled_roles(root, diagnostics, |config| match config.schedule.innovators {
        Some(layer) => layer.roles.into_iter().map(|role| role.name.as_str().to_string()).collect(),
        None => Vec::new(),
    })
}

fn scheduled_observer_roles(root: &Path, diagnostics: &mut Diagnostics) -> Vec<String> {
    get_scheduled_roles(root, diagnostics, |config| {
        config
            .schedule
            .observers
            .roles
            .into_iter()
            .map(|role| role.name.as_str().to_string())
            .collect()
    })
}

// --- Innovator validation functions ---

fn validate_innovator_role_file(path: &Path, role_dir: &Path, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };

    ensure_non_empty_string(&data, path, "role", diagnostics);

    let layer_value = get_string(&data, "layer").unwrap_or_default();
    if layer_value != "innovators" {
        diagnostics.push_error(path.display().to_string(), "layer must be 'innovators'");
    }

    match data.get("profile") {
        Some(serde_yaml::Value::Mapping(profile_map)) => {
            ensure_non_empty_string(profile_map, path, "focus", diagnostics);
            ensure_non_empty_sequence(profile_map, path, "analysis_points", diagnostics);
            ensure_non_empty_sequence(profile_map, path, "first_principles", diagnostics);
            ensure_non_empty_sequence(profile_map, path, "guiding_questions", diagnostics);
            ensure_non_empty_sequence(profile_map, path, "anti_patterns", diagnostics);
            ensure_non_empty_sequence(profile_map, path, "evidence_expectations", diagnostics);
            ensure_non_empty_sequence(profile_map, path, "proposal_quality_bar", diagnostics);
        }
        Some(_) => {
            diagnostics.push_error(path.display().to_string(), "'profile' must be a mapping");
        }
        None => {
            diagnostics.push_error(path.display().to_string(), "Missing profile section");
        }
    }

    let role_name = role_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let role_value = get_string(&data, "role").unwrap_or_default();
    if !role_value.is_empty() && role_value != role_name {
        diagnostics.push_error(
            path.display().to_string(),
            format!("role '{}' does not match directory '{}'", role_value, role_name),
        );
    }
}

fn validate_innovator_perspective(path: &Path, role_name: &str, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };

    ensure_int(&data, path, "schema_version", diagnostics, Some(1));
    ensure_non_empty_string(&data, path, "role", diagnostics);
    ensure_non_empty_string(&data, path, "focus", diagnostics);

    let role_value = get_string(&data, "role").unwrap_or_default();
    if !role_value.is_empty() && role_value != role_name {
        diagnostics.push_error(
            path.display().to_string(),
            format!("role '{}' does not match directory '{}'", role_value, role_name),
        );
    }
}

fn validate_observer_perspective(path: &Path, role_name: &str, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };
    validate_observer_perspective_data(&data, path, role_name, diagnostics);
}

fn validate_observer_perspective_data(
    data: &Mapping,
    path: &Path,
    role_name: &str,
    diagnostics: &mut Diagnostics,
) {
    ensure_int(data, path, "schema_version", diagnostics, Some(2));
    ensure_non_empty_string(data, path, "observer", diagnostics);
    ensure_datetime(data, path, "updated_at", diagnostics);
    ensure_non_empty_sequence(data, path, "goals", diagnostics);

    // rules can be empty initially
    if let Some(rules) = get_sequence(data, "rules") {
        for (idx, item) in rules.iter().enumerate() {
            if !item.is_string() {
                diagnostics.push_error(
                    path.display().to_string(),
                    format!("rules[{}] must be a string", idx),
                );
            }
        }
    } else {
        diagnostics.push_error(path.display().to_string(), "rules must be a sequence");
    }

    if let Some(ignore) = get_sequence(data, "ignore") {
        for (idx, item) in ignore.iter().enumerate() {
            if !item.is_string() {
                diagnostics.push_error(
                    path.display().to_string(),
                    format!("ignore[{}] must be a string", idx),
                );
            }
        }
    }

    if let Some(log) = get_sequence(data, "log") {
        for (idx, entry) in log.iter().enumerate() {
            if let serde_yaml::Value::Mapping(map) = entry {
                ensure_datetime(map, path, "at", diagnostics);
                ensure_non_empty_string(map, path, "summary", diagnostics);
            } else {
                diagnostics.push_error(
                    path.display().to_string(),
                    format!("log[{}] must be a mapping", idx),
                );
            }
        }
    } else if data.get("log").is_none() {
        diagnostics.push_error(path.display().to_string(), "Missing log section");
    } else {
        diagnostics.push_error(path.display().to_string(), "The 'log' field must be a sequence");
    }

    let observer_value = get_string(data, "observer").unwrap_or_default();
    if !observer_value.is_empty() && observer_value != role_name {
        diagnostics.push_error(
            path.display().to_string(),
            format!("observer '{}' does not match directory '{}'", observer_value, role_name),
        );
    }
}

fn validate_innovator_document_common_fields(
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

fn validate_innovator_proposal(path: &Path, diagnostics: &mut Diagnostics) {
    if let Some(data) = load_yaml_mapping(path, diagnostics) {
        validate_innovator_document_common_fields(&data, path, diagnostics);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_validate_event_data_valid() {
        let yaml = r#"
schema_version: 1
id: "abc123"
requirement_id: ""
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
requirement_id: "xyz789"  # Should be empty for pending
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
        // Should have errors: requirement_id must be empty in pending, evidence must have entries
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
    fn test_validate_requirement_data_valid() {
        let yaml = r#"
schema_version: 2
requires_deep_analysis: false
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
    fn test_validate_requirement_missing_requires_deep_analysis() {
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
        assert!(diagnostics.errors()[0].message.contains("requires_deep_analysis is required"));
    }

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

    #[test]
    fn test_validate_observer_perspective_valid() {
        let yaml = r#"
schema_version: 2
observer: "cli_sentinel"
updated_at: "2023-10-27"
goals: ["Monitor CLI"]
rules: ["Be nice"]
ignore: ["ignore_me"]
log:
  - at: "2023-10-27"
    summary: "Started"
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("perspective.yml");
        let mut diagnostics = Diagnostics::default();

        validate_observer_perspective_data(&data, &path, "cli_sentinel", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn test_validate_observer_perspective_invalid_dates() {
        let yaml = r#"
schema_version: 2
observer: "cli_sentinel"
updated_at: "2023-10-27"
goals: ["Monitor CLI"]
rules: []
log:
  - at: "invalid-date"
    summary: "Started"
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("perspective.yml");
        let mut diagnostics = Diagnostics::default();

        validate_observer_perspective_data(&data, &path, "cli_sentinel", &mut diagnostics);
        assert!(diagnostics.error_count() >= 1);
        let messages: Vec<_> = diagnostics.errors().iter().map(|e| &e.message).collect();
        assert!(messages.iter().any(|m| m.contains("at must be YYYY-MM-DD")));
    }

    #[test]
    fn test_validate_observer_perspective_missing_fields() {
        let yaml = r#"
schema_version: 2
observer: "cli_sentinel"
updated_at: "2023-10-27"
# Missing goals, rules
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("perspective.yml");
        let mut diagnostics = Diagnostics::default();

        validate_observer_perspective_data(&data, &path, "cli_sentinel", &mut diagnostics);
        assert!(diagnostics.error_count() > 0);
    }
    #[test]
    fn test_validate_observer_perspective_log_not_sequence() {
        let yaml = r#"
schema_version: 2
observer: "cli_sentinel"
updated_at: "2023-10-27"
goals: ["Monitor CLI"]
rules: ["Be nice"]
log: "this should be a sequence"
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("perspective.yml");
        let mut diagnostics = Diagnostics::default();

        validate_observer_perspective_data(&data, &path, "cli_sentinel", &mut diagnostics);
        assert!(diagnostics.error_count() > 0);
        let messages: Vec<_> = diagnostics.errors().iter().map(|e| &e.message).collect();
        assert!(messages.iter().any(|m| m.contains("The 'log' field must be a sequence")));
    }

    #[test]
    fn test_validate_observer_perspective_placeholder_date() {
        let yaml = r#"
schema_version: 2
observer: "cli_sentinel"
updated_at: "YYYY-MM-DD"
goals: ["Monitor CLI"]
rules: ["Be nice"]
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("perspective.yml");
        let mut diagnostics = Diagnostics::default();

        validate_observer_perspective_data(&data, &path, "cli_sentinel", &mut diagnostics);
        assert!(diagnostics.error_count() > 0);
        let messages: Vec<_> = diagnostics.errors().iter().map(|e| &e.message).collect();
        assert!(messages.iter().any(|m| m.contains("updated_at must be YYYY-MM-DD")));
    }

    #[test]
    fn test_check_placeholders_datetime() {
        let content = "updated_at: YYYY-MM-DD";
        let path = PathBuf::from("test.yml");
        let mut diagnostics = Diagnostics::default();

        check_placeholders(content, &path, &mut diagnostics);
        assert!(diagnostics.error_count() > 0);
        let messages: Vec<_> = diagnostics.errors().iter().map(|e| &e.message).collect();
        assert!(messages.iter().any(|m| m.contains("placeholder 'YYYY-MM-DD' must be replaced")));
    }
}
