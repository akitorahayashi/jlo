use serde_yaml::Mapping;
use std::fs;
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

pub fn scheduled_innovator_roles(root: &Path, diagnostics: &mut Diagnostics) -> Vec<String> {
    get_scheduled_roles(root, diagnostics, |config| match config.schedule.innovators {
        Some(layer) => layer.roles.into_iter().map(|role| role.name.as_str().to_string()).collect(),
        None => Vec::new(),
    })
}

pub fn scheduled_observer_roles(root: &Path, diagnostics: &mut Diagnostics) -> Vec<String> {
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
