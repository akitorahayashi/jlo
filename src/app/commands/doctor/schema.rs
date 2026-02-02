use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;

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
    pub layer: Layer,
    pub role: String,
    pub workstream: Option<String>,
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
            let prompt_path = layer_dir.join("prompt.yml");
            if prompt_path.exists()
                && let Some(entry) = parse_prompt(&prompt_path, layer, diagnostics)
            {
                entries.push(entry);
            }
        } else {
            for role_dir in list_subdirs(&layer_dir, diagnostics) {
                let prompt_path = role_dir.join("prompt.yml");
                if prompt_path.exists()
                    && let Some(entry) = parse_prompt(&prompt_path, layer, diagnostics)
                {
                    entries.push(entry);
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
        let change_template_path =
            inputs.jules_path.join("roles").join("narrator").join("change.yml");
        validate_changes_latest(&latest_path, &change_template_path, diagnostics);
    }

    for layer in Layer::ALL {
        let layer_dir = inputs.jules_path.join("roles").join(layer.dir_name());
        if !layer_dir.exists() {
            continue;
        }

        let contracts_path = layer_dir.join("contracts.yml");
        if contracts_path.exists() {
            validate_contracts(&contracts_path, layer, diagnostics);
        }

        if layer == Layer::Observers {
            for role_dir in list_subdirs(&layer_dir, diagnostics) {
                let role_path = role_dir.join("role.yml");
                if role_path.exists() {
                    validate_role(&role_path, &role_dir, diagnostics);
                }

                let feedback_dir = role_dir.join("feedbacks");
                if feedback_dir.exists() {
                    match fs::read_dir(&feedback_dir) {
                        Ok(entries) => {
                            for entry in entries {
                                match entry {
                                    Ok(entry) => {
                                        let path = entry.path();
                                        if path.extension().and_then(|ext| ext.to_str())
                                            == Some("yml")
                                        {
                                            validate_feedback(&path, diagnostics);
                                            check_placeholders(&path, diagnostics);
                                        }
                                    }
                                    Err(err) => {
                                        diagnostics.push_error(
                                            feedback_dir.display().to_string(),
                                            format!("Failed to read directory entry: {}", err),
                                        );
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            diagnostics.push_error(
                                feedback_dir.display().to_string(),
                                format!("Failed to read directory: {}", err),
                            );
                        }
                    }
                }
            }
        }
    }

    for workstream in inputs.workstreams {
        let ws_dir = inputs.jules_path.join("workstreams").join(workstream);
        let events_dir = ws_dir.join("events");
        for state in inputs.event_states {
            let state_dir = events_dir.join(state);
            for entry in read_yaml_files(&state_dir, diagnostics) {
                validate_event(&entry, state, inputs.event_confidence, diagnostics);
                check_placeholders(&entry, diagnostics);
            }
        }

        let issues_dir = ws_dir.join("issues");
        for label in inputs.issue_labels {
            let label_dir = issues_dir.join(label);
            for entry in read_yaml_files(&label_dir, diagnostics) {
                validate_issue(
                    &entry,
                    label,
                    inputs.issue_labels,
                    inputs.issue_priorities,
                    diagnostics,
                );
                check_placeholders(&entry, diagnostics);
            }
        }
    }
}

fn parse_prompt(path: &Path, layer: Layer, diagnostics: &mut Diagnostics) -> Option<PromptEntry> {
    let data = load_yaml_mapping(path, diagnostics)?;

    let role = get_string(&data, "role");
    let role_value = role.clone().unwrap_or_default();
    if role.as_deref().unwrap_or("").is_empty() {
        diagnostics.push_error(path.display().to_string(), "Missing role field");
    }

    let layer_field = get_string(&data, "layer").unwrap_or_default();
    if layer_field != layer.dir_name() {
        diagnostics.push_error(
            path.display().to_string(),
            format!("Layer field '{}' does not match {}", layer_field, layer.dir_name()),
        );
    }

    let contracts = get_sequence_strings(&data, "contracts");
    if contracts.is_empty() {
        diagnostics.push_error(path.display().to_string(), "Missing contracts list");
    }

    let instructions = get_sequence_strings(&data, "instructions");
    if instructions.is_empty() {
        diagnostics.push_error(path.display().to_string(), "Missing instructions list");
    }

    let workstream = get_string(&data, "workstream");
    if layer.is_single_role() {
        if workstream.is_some() {
            diagnostics.push_error(
                path.display().to_string(),
                "workstream not allowed in single-role layer",
            );
        }
    } else if workstream.as_deref().unwrap_or("").is_empty() {
        diagnostics.push_error(path.display().to_string(), "Missing workstream");
    }

    Some(PromptEntry { path: path.to_path_buf(), layer, role: role_value, workstream, contracts })
}

fn validate_event(
    path: &Path,
    state: &str,
    event_confidence: &[String],
    diagnostics: &mut Diagnostics,
) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };

    ensure_int(&data, path, "schema_version", diagnostics, Some(1));
    ensure_id(&data, path, "id", diagnostics);

    let issue_id = match get_string(&data, "issue_id") {
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

    ensure_date(&data, path, "created_at", diagnostics);
    ensure_non_empty_string(&data, path, "author_role", diagnostics);
    let allowed: Vec<&str> = event_confidence.iter().map(|value| value.as_str()).collect();
    ensure_enum(&data, path, "confidence", &allowed, diagnostics);
    ensure_non_empty_string(&data, path, "title", diagnostics);
    ensure_non_empty_string(&data, path, "statement", diagnostics);

    if let Some(evidence) = get_sequence(&data, "evidence") {
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

fn validate_issue(
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

    ensure_int(&data, path, "schema_version", diagnostics, Some(2));
    ensure_id(&data, path, "id", diagnostics);
    if get_sequence(&data, "source_events").map(|seq| seq.is_empty()).unwrap_or(true) {
        diagnostics.push_error(path.display().to_string(), "source_events must have entries");
    } else if let Some(seq) = get_sequence(&data, "source_events") {
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

    ensure_non_empty_string(&data, path, "title", diagnostics);
    ensure_non_empty_string(&data, path, "label", diagnostics);

    let label_value = get_string(&data, "label").unwrap_or_default();
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
    ensure_enum(&data, path, "priority", &allowed, diagnostics);

    ensure_non_empty_string(&data, path, "summary", diagnostics);
    ensure_non_empty_string(&data, path, "problem", diagnostics);
    ensure_non_empty_string(&data, path, "impact", diagnostics);
    ensure_non_empty_string(&data, path, "desired_outcome", diagnostics);

    if get_sequence(&data, "affected_areas").map(|seq| seq.is_empty()).unwrap_or(true) {
        diagnostics.push_error(path.display().to_string(), "affected_areas must have entries");
    }

    if get_sequence(&data, "acceptance_criteria").map(|seq| seq.is_empty()).unwrap_or(true) {
        diagnostics.push_error(path.display().to_string(), "acceptance_criteria must have entries");
    }

    if get_sequence(&data, "verification_commands").map(|seq| seq.is_empty()).unwrap_or(true) {
        diagnostics
            .push_error(path.display().to_string(), "verification_commands must have entries");
    } else if let Some(seq) = get_sequence(&data, "verification_commands") {
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

    let requires_deep = get_bool(&data, "requires_deep_analysis").unwrap_or(false);
    let deep_reason = get_string(&data, "deep_analysis_reason").unwrap_or_default();
    if requires_deep && deep_reason.trim().is_empty() {
        diagnostics.push_error(
            path.display().to_string(),
            "deep_analysis_reason required when requires_deep_analysis is true",
        );
    }
}

fn validate_feedback(path: &Path, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };

    ensure_date(&data, path, "date", diagnostics);
    ensure_non_empty_string(&data, path, "topic", diagnostics);
    ensure_non_empty_string(&data, path, "critique", diagnostics);
    ensure_non_empty_string(&data, path, "guidance", diagnostics);
    ensure_non_empty_string(&data, path, "rejected_content", diagnostics);
}

fn validate_role(path: &Path, role_dir: &Path, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };

    ensure_non_empty_string(&data, path, "role", diagnostics);
    ensure_non_empty_string(&data, path, "focus", diagnostics);

    if get_sequence(&data, "analysis_points").map(|seq| seq.is_empty()).unwrap_or(true) {
        diagnostics.push_error(path.display().to_string(), "analysis_points must have entries");
    }

    if get_sequence(&data, "learned_exclusions").is_none() {
        diagnostics.push_error(path.display().to_string(), "learned_exclusions is required");
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

fn validate_contracts(path: &Path, layer: Layer, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };

    let layer_value = get_string(&data, "layer").unwrap_or_default();
    if layer_value != layer.dir_name() {
        diagnostics.push_error(
            path.display().to_string(),
            format!("layer '{}' does not match directory '{}'", layer_value, layer.dir_name()),
        );
    }

    let prefix = get_string(&data, "branch_prefix").unwrap_or_default();
    let layer_slug = layer.dir_name().trim_end_matches('s');
    if !prefix.starts_with(&format!("jules-{}-", layer_slug)) {
        diagnostics.push_error(path.display().to_string(), "branch_prefix is invalid");
    }

    ensure_non_empty_sequence(&data, path, "constraints", diagnostics);
    ensure_non_empty_sequence(&data, path, "inputs", diagnostics);
    ensure_non_empty_sequence(&data, path, "outputs", diagnostics);
    ensure_non_empty_sequence(&data, path, "workflow", diagnostics);
}

/// Validate .jules/changes/latest.yml schema.
fn validate_changes_latest(path: &Path, template_path: &Path, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };

    // Required fields
    ensure_int(&data, path, "schema_version", diagnostics, Some(1));
    ensure_id(&data, path, "id", diagnostics);
    ensure_non_empty_string(&data, path, "created_at", diagnostics);

    // Validate range mapping
    if let Some(range) = data.get(serde_yaml::Value::String("range".to_string())) {
        if let serde_yaml::Value::Mapping(range_map) = range {
            ensure_non_empty_string(range_map, path, "from_commit", diagnostics);
            ensure_non_empty_string(range_map, path, "to_commit", diagnostics);

            // Validate selection_mode enum from template
            let allowed_modes = extract_enum_from_template(template_path, "selection_mode");
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

    // Validate diffstat mapping
    if let Some(diffstat) = data.get(serde_yaml::Value::String("diffstat".to_string())) {
        if let serde_yaml::Value::Mapping(diffstat_map) = diffstat {
            ensure_int(diffstat_map, path, "files_changed", diagnostics, None);
            ensure_int(diffstat_map, path, "insertions", diagnostics, None);
            ensure_int(diffstat_map, path, "deletions", diagnostics, None);
        } else {
            diagnostics.push_error(path.display().to_string(), "diffstat must be a mapping");
        }
    } else {
        diagnostics.push_error(path.display().to_string(), "Missing diffstat field");
    }

    // Validate commits list exists
    if get_sequence(&data, "commits").is_none() {
        diagnostics.push_error(path.display().to_string(), "Missing commits list");
    }

    // Validate changed_paths list exists
    if get_sequence(&data, "changed_paths").is_none() {
        diagnostics.push_error(path.display().to_string(), "Missing changed_paths list");
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

/// Extract allowed enum values from the change.yml template.
fn extract_enum_from_template(_template_path: &Path, field: &str) -> Vec<String> {
    // Read the template and look for the field's comment or value
    // For simplicity, we hardcode the known values from the template
    // A more robust implementation would parse the template
    match field {
        "selection_mode" => vec!["incremental".to_string(), "bootstrap".to_string()],
        _ => vec![],
    }
}

fn check_placeholders(path: &Path, diagnostics: &mut Diagnostics) {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            diagnostics
                .push_error(path.display().to_string(), format!("Failed to read file: {}", err));
            return;
        }
    };

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
