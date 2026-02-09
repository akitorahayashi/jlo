//! Setup gen command - generates install.sh and env.toml.

use crate::adapters::assets::component_catalog_embedded::EmbeddedComponentCatalog;
use crate::app::config::SetupConfig;
use crate::app::services::setup_component_resolver::SetupComponentResolver;
use crate::app::services::setup_script_generator::SetupScriptGenerator;
use crate::domain::AppError;
use crate::ports::WorkspaceStore;

/// Execute the setup gen command.
///
/// Reads `.jules/setup/tools.yml`, resolves dependencies, and generates:
/// - `.jules/setup/install.sh` - Installation script
/// - `.jules/setup/env.toml` - Environment variables
///
/// Returns the list of resolved component names in installation order.
pub fn execute(store: &impl WorkspaceStore) -> Result<Vec<String>, AppError> {
    let setup_dir = ".jules/setup";
    let tools_yml = ".jules/setup/tools.yml";

    if !store.file_exists(setup_dir) {
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

    // Initialize services
    let catalog = EmbeddedComponentCatalog::new()?;

    // Resolve dependencies
    let components = SetupComponentResolver::resolve(&config.tools, &catalog)?;

    // Generate install script
    let script_content = SetupScriptGenerator::generate_install_script(&components);
    let install_sh = ".jules/setup/install.sh";
    store.write_file(install_sh, &script_content)?;

    // Make executable
    store.set_executable(install_sh)?;

    // Generate/merge env.toml
    let env_toml_path = ".jules/setup/env.toml";
    let existing_content =
        if store.file_exists(env_toml_path) { Some(store.read_file(env_toml_path)?) } else { None };
    let env_content =
        SetupScriptGenerator::merge_env_toml(&components, existing_content.as_deref())?;
    store.write_file(env_toml_path, &env_content)?;

    // Return component names
    Ok(components.iter().map(|c| c.name.to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory_workspace_store::MemoryWorkspaceStore;
    use crate::ports::WorkspaceStore;

    #[test]
    fn fails_if_not_initialized() {
        let store = MemoryWorkspaceStore::new();

        let result = execute(&store);

        assert!(matches!(result, Err(AppError::SetupNotInitialized)));
    }

    #[test]
    fn fails_if_tools_yml_missing() {
        let store = MemoryWorkspaceStore::new();
        store.write_file(".jules/setup/placeholder", "").unwrap();

        let result = execute(&store);

        assert!(matches!(result, Err(AppError::SetupConfigMissing)));
    }

    #[test]
    fn fails_if_no_tools_specified() {
        let store = MemoryWorkspaceStore::new();
        store.write_file(".jules/setup/tools.yml", "tools: []").unwrap();

        let result = execute(&store);

        assert!(result.is_err());
    }

    #[test]
    fn generates_install_script() {
        let store = MemoryWorkspaceStore::new();
        store.write_file(".jules/setup/tools.yml", "tools:\n  - just").unwrap();

        let result = execute(&store).unwrap();

        assert!(result.contains(&"just".to_string()));

        let install_sh = ".jules/setup/install.sh";
        assert!(store.file_exists(install_sh));

        let content = store.read_file(install_sh).unwrap();
        assert!(content.starts_with("#!/usr/bin/env bash"));
        assert!(content.contains("just"));
    }
}
