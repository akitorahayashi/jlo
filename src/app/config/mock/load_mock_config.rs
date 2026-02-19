//! Mock configuration loading from repository.

use std::collections::HashMap;
use std::path::Path;

use crate::app::config::load_config;
use crate::domain::config::mock_parse::{extract_branch_prefix, extract_issue_labels};
use crate::domain::jules_paths;
use crate::domain::{AppError, ConfigError, Layer, MockConfig};
use crate::ports::RepositoryFilesystem;

use super::mock_tag::resolve_mock_tag;

fn load_branch_prefix_for_layer(layer: Layer) -> Result<String, AppError> {
    let catalog_path = format!("{}/contracts.yml", layer.dir_name());
    let content = crate::adapters::catalogs::prompt_assemble_assets::read_prompt_assemble_asset(
        &catalog_path,
    )
    .ok_or_else(|| -> AppError {
        ConfigError::Invalid(format!(
            "Missing contracts for layer '{}' in embedded catalog: prompt-assemble://{}",
            layer.dir_name(),
            catalog_path
        ))
        .into()
    })?;

    extract_branch_prefix(&content).map_err(|e| {
        AppError::from(ConfigError::Invalid(format!(
            "Invalid contracts for layer '{}': {}",
            layer.dir_name(),
            e
        )))
    })
}

/// Load mock configuration from repository files.
pub fn load_mock_config<W: RepositoryFilesystem>(
    jules_path: &Path,
    repository: &W,
) -> Result<MockConfig, AppError> {
    let run_config = load_config(jules_path, repository)?;

    let mut branch_prefixes = HashMap::new();
    for layer in Layer::ALL {
        let prefix = load_branch_prefix_for_layer(layer)?;
        branch_prefixes.insert(layer, prefix);
    }

    let labels_path = jules_paths::github_labels(jules_path);
    let labels_path_str = labels_path
        .to_str()
        .ok_or_else(|| AppError::InvalidPath("Invalid labels path".to_string()))?;
    let labels_content = repository.read_file(labels_path_str).map_err(|_| {
        ConfigError::Invalid(format!(
            "Missing github-labels.json for mock mode: {}",
            labels_path.display()
        ))
    })?;
    let issue_labels = extract_issue_labels(&labels_content)?;
    if issue_labels.is_empty() {
        return Err(ConfigError::Invalid(format!(
            "No issue labels defined in github-labels.json: {}",
            labels_path.display()
        ))
        .into());
    }

    let mock_tag = resolve_mock_tag()?;

    Ok(MockConfig {
        mock_tag,
        branch_prefixes,
        jlo_target_branch: run_config.run.jlo_target_branch,
        jules_worker_branch: run_config.run.jules_worker_branch,
        issue_labels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_branch_prefix_for_innovators_reads_from_embedded_catalog() {
        let prefix = load_branch_prefix_for_layer(Layer::Innovators).unwrap();

        assert!(!prefix.is_empty(), "innovators branch_prefix should be non-empty");
    }
}
