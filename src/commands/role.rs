//! Role command: print scheduler prompt material for roles.

use std::io::{BufRead, IsTerminal};

use dialoguer::Select;

use crate::error::AppError;
use crate::scaffold;
use crate::workspace::Workspace;

/// Execute role command interactively: show menu and print role config.
pub fn execute() -> Result<String, AppError> {
    // Early validation: check workspace exists BEFORE showing menu
    let workspace = Workspace::current()?;
    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // Discover existing roles in workspace
    let existing_roles = workspace.discover_roles()?;

    // Get built-in roles
    let builtin_roles = scaffold::role_definitions();

    // Build menu: existing roles first, then missing built-ins
    let mut menu_items = Vec::new();
    let mut menu_metadata: Vec<(String, bool)> = Vec::new(); // (role_id, is_existing)

    for role_id in &existing_roles {
        menu_items.push(format!("{} (existing)", role_id));
        menu_metadata.push((role_id.clone(), true));
    }

    for role in builtin_roles {
        if !existing_roles.contains(&role.id.to_string()) {
            menu_items.push(format!("{} (built-in)", role.id));
            menu_metadata.push((role.id.to_string(), false));
        }
    }

    if menu_items.is_empty() {
        return Err(AppError::config_error(
            "No roles available. Use 'jo role' after creating roles manually.",
        ));
    }

    // Show menu and get selection
    let selection = if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        Select::new()
            .with_prompt("Select a role")
            .items(&menu_items)
            .default(0)
            .interact()
            .map_err(|err| AppError::config_error(format!("Role selection failed: {err}")))?
    } else {
        // Non-interactive mode: read from stdin
        let mut input = String::new();
        let mut stdin = std::io::stdin().lock();
        stdin
            .read_line(&mut input)
            .map_err(|err| AppError::config_error(format!("Role selection failed: {err}")))?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(AppError::config_error(
                "Role selection requires input when no TTY is available.",
            ));
        }

        // Parse as 1-based index or role name
        if let Ok(index) = trimmed.parse::<usize>() {
            if index == 0 || index > menu_items.len() {
                return Err(AppError::config_error("Role selection index out of range."));
            }
            index - 1
        } else {
            // Try to find by role_id
            menu_metadata
                .iter()
                .position(|(id, _)| id == trimmed)
                .ok_or_else(|| AppError::config_error(format!("Role '{}' not found", trimmed)))?
        }
    };

    let (role_id, is_existing) = &menu_metadata[selection];

    // If built-in role doesn't exist, scaffold it first
    if !is_existing {
        let builtin = scaffold::role_definition(role_id).ok_or_else(|| {
            AppError::config_error(format!("Built-in role '{}' not found", role_id))
        })?;
        workspace.scaffold_role(
            role_id,
            builtin.role_yaml,
            Some(builtin.prompt_yaml),
            builtin.has_notes,
        )?;
    }

    // Print the scheduler prompt as-is for copy/paste into the scheduler.
    let prompt = workspace.read_role_prompt(role_id)?;
    println!("{}", prompt);

    Ok(role_id.clone())
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
    fn role_fails_early_without_workspace() {
        with_temp_cwd(|_dir| {
            // This should be tested via integration test since it requires stdin
            // Unit test just validates the workspace check logic
            let workspace = Workspace::current().unwrap();
            assert!(!workspace.exists());
        });
    }

    #[test]
    #[serial]
    fn role_discovers_existing() {
        with_temp_cwd(|_dir| {
            init::execute().unwrap();
            let workspace = Workspace::current().unwrap();
            workspace.scaffold_role("custom-role", "Custom config content", None, true).unwrap();

            let roles = workspace.discover_roles().unwrap();
            assert!(roles.contains(&"custom-role".to_string()));
        });
    }

    #[test]
    #[serial]
    fn role_config_composition_works() {
        with_temp_cwd(|_dir| {
            init::execute().unwrap();
            let workspace = Workspace::current().unwrap();
            workspace
                .scaffold_role("test-role", "Test config fragment", Some("prompt"), true)
                .unwrap();

            let prompt = workspace.read_role_prompt("test-role").unwrap();
            assert!(prompt.contains("prompt"));
        });
    }
}
