//! Role command: scaffold a role workspace under `.jules/roles/`.

use std::io::{BufRead, IsTerminal};

use dialoguer::{Input, Select};

use crate::error::AppError;
use crate::scaffold;
use crate::workspace::{Workspace, is_valid_role_id};

/// Options for the role command.
pub struct RoleOptions<'a> {
    /// The role identifier.
    pub role_id: &'a str,
}

/// Execute the role command.
pub fn execute(options: &RoleOptions<'_>) -> Result<(), AppError> {
    let workspace = Workspace::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    if !is_valid_role_id(options.role_id) {
        return Err(AppError::InvalidRoleId(options.role_id.to_string()));
    }

    if workspace.role_exists(options.role_id) {
        // Role already exists - not an error, just skip
        return Ok(());
    }

    workspace.create_role(options.role_id)?;

    Ok(())
}

/// Execute role creation via an interactive selection menu.
pub fn execute_interactive() -> Result<String, AppError> {
    let roles = scaffold::role_definitions();
    if roles.is_empty() {
        return Err(AppError::config_error("No built-in roles are available."));
    }

    let mut choices: Vec<String> = roles
        .iter()
        .map(|role| {
            if role.summary.is_empty() {
                format!("{} — {}", role.id, role.title)
            } else {
                format!("{} — {}", role.id, role.summary)
            }
        })
        .collect();
    choices.push("custom — enter a role id".to_string());

    let role_id = if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        let selection = Select::new()
            .with_prompt("Select a role to add")
            .items(&choices)
            .default(0)
            .interact()
            .map_err(|err| AppError::config_error(format!("Role selection failed: {err}")))?;
        if selection == roles.len() {
            let input: String = Input::new()
                .with_prompt("Enter role id")
                .interact()
                .map_err(|err| AppError::config_error(format!("Role input failed: {err}")))?;
            input.trim().to_string()
        } else {
            roles[selection].id.clone()
        }
    } else {
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

        if let Ok(index) = trimmed.parse::<usize>() {
            if index == 0 || index > roles.len() {
                return Err(AppError::config_error("Role selection index out of range."));
            }
            roles[index - 1].id.clone()
        } else {
            let role = roles.iter().find(|role| role.id == trimmed);
            role.map(|role| role.id.clone()).unwrap_or_else(|| trimmed.to_string())
        }
    };

    execute(&RoleOptions { role_id: &role_id })?;

    Ok(role_id)
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
        F: FnOnce() -> R,
    {
        let dir = TempDir::new().expect("failed to create temp dir");
        let original = env::current_dir().expect("failed to get cwd");
        env::set_current_dir(dir.path()).expect("failed to set cwd");
        let result = f();
        env::set_current_dir(original).expect("failed to restore cwd");
        result
    }

    #[test]
    #[serial]
    fn role_fails_without_workspace() {
        with_temp_cwd(|| {
            let options = RoleOptions { role_id: "value" };
            let err = execute(&options).expect_err("role should fail");
            assert!(matches!(err, AppError::WorkspaceNotFound));
        });
    }

    #[test]
    #[serial]
    fn role_creates_directory() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();

            let options = RoleOptions { role_id: "value" };
            execute(&options).expect("role should succeed");

            let cwd = env::current_dir().unwrap();
            let role_dir = cwd.join(".jules/roles/value");
            assert!(role_dir.exists());
            assert!(role_dir.join("charter.md").exists());
            assert!(role_dir.join("direction.md").exists());
            assert!(role_dir.join("sessions").exists());
        });
    }

    #[test]
    #[serial]
    fn role_fails_for_invalid_id() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();

            let options = RoleOptions { role_id: "invalid/id" };
            let err = execute(&options).expect_err("role should fail");
            assert!(matches!(err, AppError::InvalidRoleId(_)));
        });
    }

    #[test]
    #[serial]
    fn role_is_idempotent() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();

            let options = RoleOptions { role_id: "value" };
            execute(&options).expect("first role should succeed");
            execute(&options).expect("second role should succeed");
        });
    }
}
