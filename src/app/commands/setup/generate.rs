//! Setup gen command - generates install.sh and env.toml.

use std::path::Path;

use crate::adapters::EmbeddedCatalog;
use crate::domain::AppError;
use crate::domain::setup::SetupConfig;
use crate::services::{Generator, Resolver};

/// Execute the setup gen command.
///
/// Reads `.jules/setup/tools.yml`, resolves dependencies, and generates:
/// - `.jules/setup/install.sh` - Installation script
/// - `.jules/setup/env.toml` - Environment variables
///
/// Returns the list of resolved component names in installation order.
pub fn execute(path: Option<&Path>) -> Result<Vec<String>, AppError> {
    let target = match path {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir()?,
    };

    let setup_dir = target.join(".jules").join("setup");

    if !setup_dir.exists() {
        return Err(AppError::SetupNotInitialized);
    }

    let tools_yml = setup_dir.join("tools.yml");
    if !tools_yml.exists() {
        return Err(AppError::SetupConfigMissing);
    }

    // Load configuration
    let content = std::fs::read_to_string(&tools_yml)?;
    let config: SetupConfig = serde_yaml::from_str(&content)
        .map_err(|e| AppError::config_error(format!("Invalid tools.yml: {}", e)))?;

    if config.tools.is_empty() {
        return Err(AppError::config_error(
            "No tools specified in tools.yml. Add tools to the 'tools' list.",
        ));
    }

    // Initialize services
    let catalog = EmbeddedCatalog::new()?;

    // Resolve dependencies
    let components = Resolver::resolve(&config.tools, &catalog)?;

    // Generate install script
    let script_content = Generator::generate_install_script(&components);
    let install_sh = setup_dir.join("install.sh");
    std::fs::write(&install_sh, &script_content)?;

    // Make executable (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&install_sh)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&install_sh, perms)?;
    }

    // Generate/merge env.toml
    let env_toml_path = setup_dir.join("env.toml");
    let existing_path = if env_toml_path.exists() { Some(env_toml_path.as_path()) } else { None };
    let env_content = Generator::merge_env_toml(&components, existing_path)?;
    std::fs::write(&env_toml_path, &env_content)?;

    // Return component names
    Ok(components.iter().map(|c| c.name.clone()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn setup_initialized_workspace(path: &Path) {
        let setup_dir = path.join(".jules/setup");
        std::fs::create_dir_all(&setup_dir).unwrap();
    }

    #[test]
    fn fails_if_not_initialized() {
        let temp = tempdir().unwrap();

        let result = execute(Some(temp.path()));

        assert!(matches!(result, Err(AppError::SetupNotInitialized)));
    }

    #[test]
    fn fails_if_tools_yml_missing() {
        let temp = tempdir().unwrap();
        setup_initialized_workspace(temp.path());

        let result = execute(Some(temp.path()));

        assert!(matches!(result, Err(AppError::SetupConfigMissing)));
    }

    #[test]
    fn fails_if_no_tools_specified() {
        let temp = tempdir().unwrap();
        setup_initialized_workspace(temp.path());

        let tools_yml = temp.path().join(".jules/setup/tools.yml");
        let mut file = std::fs::File::create(&tools_yml).unwrap();
        writeln!(file, "tools: []").unwrap();

        let result = execute(Some(temp.path()));

        assert!(result.is_err());
    }

    #[test]
    fn generates_install_script() {
        let temp = tempdir().unwrap();
        setup_initialized_workspace(temp.path());

        let tools_yml = temp.path().join(".jules/setup/tools.yml");
        let mut file = std::fs::File::create(&tools_yml).unwrap();
        writeln!(file, "tools:").unwrap();
        writeln!(file, "  - just").unwrap();

        let result = execute(Some(temp.path())).unwrap();

        assert!(result.contains(&"just".to_string()));

        let install_sh = temp.path().join(".jules/setup/install.sh");
        assert!(install_sh.exists());

        let content = std::fs::read_to_string(&install_sh).unwrap();
        assert!(content.starts_with("#!/usr/bin/env bash"));
        assert!(content.contains("just"));
    }

    #[cfg(unix)]
    #[test]
    fn install_script_is_executable() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir().unwrap();
        setup_initialized_workspace(temp.path());

        let tools_yml = temp.path().join(".jules/setup/tools.yml");
        let mut file = std::fs::File::create(&tools_yml).unwrap();
        writeln!(file, "tools:").unwrap();
        writeln!(file, "  - just").unwrap();

        execute(Some(temp.path())).unwrap();

        let install_sh = temp.path().join(".jules/setup/install.sh");
        let metadata = std::fs::metadata(&install_sh).unwrap();
        let mode = metadata.permissions().mode();
        assert!(mode & 0o111 != 0, "install.sh should be executable");
    }
}
