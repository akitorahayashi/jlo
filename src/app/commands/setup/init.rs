//! Setup init command - creates .jules/setup/ structure.

use std::path::Path;

use crate::domain::AppError;

const TOOLS_YML_TEMPLATE: &str = r#"# jlo setup configuration
# List the tools you want to install

tools:
  # - just
  # - swift
  # - uv
"#;

const GITIGNORE_CONTENT: &str = r#"# Ignore environment configuration with secrets
env.toml
"#;

/// Execute the setup init command.
///
/// Creates `.jules/setup/` directory with:
/// - `tools.yml` - Tool selection configuration
/// - `.gitignore` - Ignores env.toml
pub fn execute(path: Option<&Path>) -> Result<(), AppError> {
    let target = path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    let setup_dir = target.join(".jules").join("setup");

    if setup_dir.exists() {
        return Err(AppError::config_error(format!(
            "Setup already initialized: {}",
            setup_dir.display()
        )));
    }

    // Create directory structure
    std::fs::create_dir_all(&setup_dir)?;

    // Write tools.yml template
    let tools_yml = setup_dir.join("tools.yml");
    std::fs::write(&tools_yml, TOOLS_YML_TEMPLATE)?;

    // Write .gitignore
    let gitignore = setup_dir.join(".gitignore");
    std::fs::write(&gitignore, GITIGNORE_CONTENT)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_setup_directory() {
        let temp = tempdir().unwrap();
        let path = temp.path();

        execute(Some(path)).unwrap();

        assert!(path.join(".jules/setup").exists());
        assert!(path.join(".jules/setup/tools.yml").exists());
        assert!(path.join(".jules/setup/.gitignore").exists());
    }

    #[test]
    fn fails_if_already_exists() {
        let temp = tempdir().unwrap();
        let path = temp.path();

        execute(Some(path)).unwrap();
        let result = execute(Some(path));

        assert!(result.is_err());
    }

    #[test]
    fn tools_yml_has_correct_content() {
        let temp = tempdir().unwrap();
        let path = temp.path();

        execute(Some(path)).unwrap();

        let content = std::fs::read_to_string(path.join(".jules/setup/tools.yml")).unwrap();
        assert!(content.contains("tools:"));
        assert!(content.contains("# - just"));
    }
}
