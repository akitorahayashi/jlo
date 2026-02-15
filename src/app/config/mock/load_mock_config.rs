//! Mock configuration loading from repository.

use std::collections::HashMap;
use std::path::Path;

use crate::app::config::load_config;
use crate::domain::config::mock_parse::{extract_branch_prefix, extract_issue_labels};
use crate::domain::{AppError, Layer, MockConfig, RunOptions};
use crate::domain::{layers, workstations};
use crate::ports::RepositoryFilesystem;

use super::mock_tag::resolve_mock_tag;

fn load_branch_prefix_for_layer<W: RepositoryFilesystem>(
    jules_path: &Path,
    layer: Layer,
    repository: &W,
) -> Result<String, AppError> {
    let contracts_path = layers::paths::contracts(jules_path, layer);
    let contracts_path_str = contracts_path
        .to_str()
        .ok_or_else(|| AppError::InvalidPath("Invalid contracts path".to_string()))?;

    let content = repository.read_file(contracts_path_str).map_err(|_| {
        AppError::InvalidConfig(format!(
            "Missing contracts file for layer '{}' at {}",
            layer.dir_name(),
            contracts_path.display()
        ))
    })?;

    extract_branch_prefix(&content).map_err(|e| {
        AppError::InvalidConfig(format!(
            "Invalid contracts file for layer '{}' at {}: {}",
            layer.dir_name(),
            contracts_path.display(),
            e
        ))
    })
}

/// Load mock configuration from repository files.
pub fn load_mock_config<W: RepositoryFilesystem>(
    jules_path: &Path,
    _options: &RunOptions,
    repository: &W,
) -> Result<MockConfig, AppError> {
    let run_config = load_config(jules_path, repository)?;

    let mut branch_prefixes = HashMap::new();
    for layer in Layer::ALL {
        let prefix = load_branch_prefix_for_layer(jules_path, layer, repository)?;
        branch_prefixes.insert(layer, prefix);
    }

    let labels_path = workstations::paths::github_labels(jules_path);
    let labels_path_str = labels_path
        .to_str()
        .ok_or_else(|| AppError::InvalidPath("Invalid labels path".to_string()))?;
    let labels_content = repository.read_file(labels_path_str).map_err(|_| {
        AppError::InvalidConfig(format!(
            "Missing github-labels.json for mock mode: {}",
            labels_path.display()
        ))
    })?;
    let issue_labels = extract_issue_labels(&labels_content)?;
    if issue_labels.is_empty() {
        return Err(AppError::InvalidConfig(format!(
            "No issue labels defined in github-labels.json: {}",
            labels_path.display()
        )));
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
    use crate::testing::TestStore;

    #[test]
    fn load_branch_prefix_for_innovators_uses_contracts_yml() {
        let repository = TestStore::new().with_file(
            ".jules/layers/innovators/contracts.yml",
            "branch_prefix: jules-innovator-\n",
        );

        let prefix =
            load_branch_prefix_for_layer(Path::new(".jules"), Layer::Innovators, &repository)
                .unwrap();

        assert_eq!(prefix, "jules-innovator-");
    }
}
