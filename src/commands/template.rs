//! Template command: scaffold new custom roles based on layer archetypes.

use std::io::{BufRead, IsTerminal};

use dialoguer::Select;

use crate::error::AppError;
use crate::generator;
use crate::layers::Layer;
use crate::workspace::Workspace;

/// Execute the template command.
///
/// Creates a new role directory under the specified layer with
/// pre-filled role.yml and prompt.yml based on the layer archetype.
pub fn execute(layer_arg: Option<&str>, role_name_arg: Option<&str>) -> Result<String, AppError> {
    let workspace = Workspace::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // Resolve layer
    let layer = match layer_arg {
        Some(name) => {
            Layer::from_dir_name(name).ok_or_else(|| AppError::InvalidLayer(name.to_string()))?
        }
        None => select_layer()?,
    };

    // Get role name
    let role_name = match role_name_arg {
        Some(name) => name.to_string(),
        None => prompt_role_name()?,
    };

    // Validate role name
    if role_name.is_empty()
        || role_name.contains('/')
        || role_name.contains('\\')
        || role_name == "."
        || role_name == ".."
        || !role_name.chars().all(|c| c.is_alphanumeric() || c == '-')
    {
        return Err(AppError::InvalidRoleId(role_name));
    }

    // Check if role already exists
    if workspace.role_exists_in_layer(layer, &role_name) {
        return Err(AppError::RoleExists { role: role_name, layer: layer.dir_name().to_string() });
    }

    // Generate role.yml and prompt.yml content
    let role_yaml = generator::generate_role_yaml(&role_name, layer);
    let prompt_yaml = generator::generate_prompt_yaml_template(&role_name, layer);

    // Determine if this layer type gets notes/
    let has_notes = matches!(layer, Layer::Observers);

    // Scaffold the role
    workspace.scaffold_role_in_layer(
        layer,
        &role_name,
        &role_yaml,
        Some(&prompt_yaml),
        has_notes,
    )?;

    Ok(format!("{}/{}", layer.dir_name(), role_name))
}

/// Interactive layer selection.
fn select_layer() -> Result<Layer, AppError> {
    let items: Vec<String> =
        Layer::ALL.iter().map(|l| format!("{} - {}", l.display_name(), l.description())).collect();

    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        let selection = Select::new()
            .with_prompt("Select a layer")
            .items(&items)
            .default(0)
            .interact()
            .map_err(|e| AppError::config_error(format!("Layer selection failed: {}", e)))?;

        Ok(Layer::ALL[selection])
    } else {
        // Non-interactive: read from stdin
        let mut input = String::new();
        let mut stdin = std::io::stdin().lock();
        stdin
            .read_line(&mut input)
            .map_err(|e| AppError::config_error(format!("Failed to read layer: {}", e)))?;

        let trimmed = input.trim();

        // Try as 1-based index
        if let Ok(index) = trimmed.parse::<usize>()
            && index >= 1
            && index <= Layer::ALL.len()
        {
            return Ok(Layer::ALL[index - 1]);
        }

        // Try as layer name
        Layer::from_dir_name(trimmed).ok_or_else(|| AppError::InvalidLayer(trimmed.to_string()))
    }
}

/// Prompt for role name interactively.
fn prompt_role_name() -> Result<String, AppError> {
    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        print!("Enter role name: ");
        use std::io::Write;
        std::io::stdout().flush().ok();
    }

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| AppError::config_error(format!("Failed to read role name: {}", e)))?;

    let name = input.trim().to_string();
    if name.is_empty() {
        return Err(AppError::config_error("Role name cannot be empty"));
    }

    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::init;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    fn with_temp_cwd<F, R>(f: F) -> R
    where
        F: FnOnce(&TempDir) -> R,
    {
        let dir = TempDir::new().expect("failed to create temp dir");
        let original = env::current_dir().expect("failed to get cwd");
        env::set_current_dir(dir.path()).expect("failed to set cwd");
        let result = f(&dir);
        env::set_current_dir(&original).expect("failed to restore cwd");
        result
    }

    #[test]
    #[serial]
    fn template_fails_without_workspace() {
        with_temp_cwd(|_dir| {
            let err = execute(Some("observers"), Some("test")).expect_err("should fail");
            assert!(matches!(err, AppError::WorkspaceNotFound));
        });
    }

    #[test]
    #[serial]
    fn template_creates_new_role() {
        with_temp_cwd(|_dir| {
            init::execute().expect("init should succeed");

            let result = execute(Some("observers"), Some("custom-role")).expect("should succeed");
            assert_eq!(result, "observers/custom-role");

            let workspace = Workspace::current().unwrap();
            assert!(workspace.role_exists_in_layer(Layer::Observers, "custom-role"));
        });
    }

    #[test]
    #[serial]
    fn template_observer_role_has_notes() {
        with_temp_cwd(|_dir| {
            init::execute().expect("init should succeed");

            execute(Some("observers"), Some("my-observer")).expect("should succeed");

            let cwd = env::current_dir().unwrap();
            let notes_dir = cwd.join(".jules/roles/observers/my-observer/notes");
            assert!(notes_dir.exists());
        });
    }

    #[test]
    #[serial]
    fn template_implementer_role_has_no_notes() {
        with_temp_cwd(|_dir| {
            init::execute().expect("init should succeed");

            execute(Some("implementers"), Some("my-impl")).expect("should succeed");

            let cwd = env::current_dir().unwrap();
            let notes_dir = cwd.join(".jules/roles/implementers/my-impl/notes");
            assert!(!notes_dir.exists());
        });
    }

    #[test]
    #[serial]
    fn template_fails_for_invalid_layer() {
        with_temp_cwd(|_dir| {
            init::execute().expect("init should succeed");

            let err = execute(Some("invalid"), Some("test")).expect_err("should fail");
            assert!(matches!(err, AppError::InvalidLayer(_)));
        });
    }

    #[test]
    #[serial]
    fn template_fails_for_existing_role() {
        with_temp_cwd(|_dir| {
            init::execute().expect("init should succeed");

            // taxonomy already exists as a built-in
            let err = execute(Some("observers"), Some("taxonomy")).expect_err("should fail");
            assert!(matches!(err, AppError::RoleExists { .. }));
        });
    }

    #[test]
    #[serial]
    fn template_fails_for_invalid_role_name() {
        with_temp_cwd(|_dir| {
            init::execute().expect("init should succeed");

            let err = execute(Some("observers"), Some("invalid/name")).expect_err("should fail");
            assert!(matches!(err, AppError::InvalidRoleId(_)));
        });
    }
}
