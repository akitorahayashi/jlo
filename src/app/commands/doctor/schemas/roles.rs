use serde_yaml::Mapping;
use std::path::Path;

use crate::app::commands::doctor::diagnostics::Diagnostics;
use crate::app::commands::doctor::yaml::{
    ensure_non_empty_sequence, ensure_non_empty_string, get_sequence, get_string, load_yaml_mapping,
};

pub fn validate_role_file(path: &Path, role_dir: &Path, diagnostics: &mut Diagnostics) {
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
    validate_constraint(data, path, diagnostics);

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

pub fn validate_innovator_role_file(path: &Path, role_dir: &Path, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };

    ensure_non_empty_string(&data, path, "role", diagnostics);

    let layer_value = get_string(&data, "layer").unwrap_or_default();
    if layer_value != "innovators" {
        diagnostics.push_error(path.display().to_string(), "layer must be 'innovators'");
    }
    validate_constraint(&data, path, diagnostics);

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

fn validate_constraint(data: &Mapping, path: &Path, diagnostics: &mut Diagnostics) {
    match data.get("constraint") {
        Some(serde_yaml::Value::Sequence(items)) => {
            for (index, item) in items.iter().enumerate() {
                if !item.is_string() {
                    diagnostics.push_error(
                        path.display().to_string(),
                        format!("constraint[{}] must be a string", index),
                    );
                }
            }
        }
        Some(_) => {
            diagnostics.push_error(path.display().to_string(), "'constraint' must be a sequence");
        }
        None => {
            diagnostics.push_error(path.display().to_string(), "Missing constraint section");
        }
    }
}
