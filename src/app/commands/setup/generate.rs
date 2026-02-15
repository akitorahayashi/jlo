//! Setup gen command - generates install.sh, vars.toml, and secrets.toml.

use crate::adapters::catalogs::EmbeddedSetupComponentCatalog;
use crate::app::config::SetupConfig;
use crate::domain::AppError;
use crate::domain::setup::artifact_generator;
use crate::domain::setup::dependency_graph::DependencyGraph;
use crate::ports::RepositoryFilesystem;

/// Execute the setup gen command.
///
/// Reads `.jlo/setup/tools.yml`, resolves dependencies, and generates:
/// - `.jlo/setup/install.sh` - Installation script
/// - `.jlo/setup/vars.toml` - Non-secret environment variables
/// - `.jlo/setup/secrets.toml` - Secret environment variables
///
/// Returns the list of resolved component names in installation order.
pub fn execute(store: &impl RepositoryFilesystem) -> Result<Vec<String>, AppError> {
    let jlo_setup = ".jlo/setup";
    let tools_yml = ".jlo/setup/tools.yml";

    if !store.file_exists(jlo_setup) {
        return Err(AppError::SetupNotInitialized);
    }

    if !store.file_exists(tools_yml) {
        return Err(AppError::SetupConfigMissing);
    }

    // Load configuration
    let content = store.read_file(tools_yml)?;
    let config: SetupConfig = serde_yaml::from_str(&content)
        .map_err(|e| AppError::ParseError { what: "tools.yml".into(), details: e.to_string() })?;

    if config.tools.is_empty() {
        return Err(AppError::Validation(
            "No tools specified in tools.yml. Add tools to the 'tools' list.".into(),
        ));
    }

    // Resolve dependencies
    let catalog = EmbeddedSetupComponentCatalog::new()?;
    let components = DependencyGraph::resolve(&config.tools, &catalog)?;

    // Generate install script
    let script_content = artifact_generator::generate_install_script(&components);
    let install_sh = ".jlo/setup/install.sh";
    store.write_file(install_sh, &script_content)?;
    store.set_executable(install_sh)?;

    // Generate/merge vars.toml and secrets.toml
    let vars_toml_path = ".jlo/setup/vars.toml";
    let secrets_toml_path = ".jlo/setup/secrets.toml";
    let existing_vars =
        store.file_exists(vars_toml_path).then(|| store.read_file(vars_toml_path)).transpose()?;
    let existing_secrets = store
        .file_exists(secrets_toml_path)
        .then(|| store.read_file(secrets_toml_path))
        .transpose()?;
    let env_artifacts = artifact_generator::merge_env_artifacts(
        &components,
        existing_vars.as_deref(),
        existing_secrets.as_deref(),
    )?;
    store.write_file(vars_toml_path, &env_artifacts.vars_toml)?;
    store.write_file(secrets_toml_path, &env_artifacts.secrets_toml)?;

    Ok(components.iter().map(|c| c.name.to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::RepositoryFilesystem;
    use crate::testing::TestStore;

    #[test]
    fn fails_if_not_initialized() {
        let store = TestStore::new();

        let result = execute(&store);

        assert!(matches!(result, Err(AppError::SetupNotInitialized)));
    }

    #[test]
    fn fails_if_tools_yml_missing() {
        let store = TestStore::new();
        store.write_file(".jlo/setup/placeholder", "").unwrap();

        let result = execute(&store);

        assert!(matches!(result, Err(AppError::SetupConfigMissing)));
    }

    #[test]
    fn fails_if_no_tools_specified() {
        let store = TestStore::new();
        store.write_file(".jlo/setup/tools.yml", "tools: []").unwrap();

        let result = execute(&store);

        assert!(result.is_err());
    }

    #[test]
    fn generates_install_script_and_env_files_in_control_plane() {
        let store = TestStore::new();
        store.write_file(".jlo/setup/tools.yml", "tools:\n  - just").unwrap();

        let result = execute(&store).unwrap();

        assert!(result.contains(&"just".to_string()));

        let install_sh = ".jlo/setup/install.sh";
        assert!(store.file_exists(install_sh));

        let content = store.read_file(install_sh).unwrap();
        assert!(content.starts_with("#!/usr/bin/env bash"));
        assert!(content.contains("just"));

        assert!(store.file_exists(".jlo/setup/vars.toml"));
        assert!(store.file_exists(".jlo/setup/secrets.toml"));
    }
}
