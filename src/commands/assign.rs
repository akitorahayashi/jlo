//! Assign command: generate prompt and copy to clipboard.

use arboard::Clipboard;

use crate::error::AppError;
use crate::generator;
use crate::workspace::Workspace;

/// Execute the assign command.
///
/// Generates a prompt for the specified role with optional path assignments
/// and copies it to the system clipboard.
pub fn execute(role_query: &str, paths: &[String]) -> Result<String, AppError> {
    let workspace = Workspace::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // Find the role using fuzzy matching
    let role = workspace
        .find_role_fuzzy(role_query)?
        .ok_or_else(|| AppError::RoleNotFound(role_query.to_string()))?;

    // Generate the prompt YAML
    let yaml = generator::generate_prompt_yaml(&role.id, role.layer, paths)
        .map_err(|e| AppError::config_error(format!("Failed to generate prompt: {}", e)))?;

    // Copy to clipboard
    let mut clipboard = Clipboard::new().map_err(|e| AppError::ClipboardError(format!("{}", e)))?;

    clipboard.set_text(&yaml).map_err(|e| AppError::ClipboardError(format!("{}", e)))?;

    Ok(role.id)
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
    fn assign_fails_without_workspace() {
        with_temp_cwd(|_dir| {
            let err = execute("taxonomy", &[]).expect_err("should fail");
            assert!(matches!(err, AppError::WorkspaceNotFound));
        });
    }

    #[test]
    #[serial]
    fn assign_fails_for_unknown_role() {
        with_temp_cwd(|_dir| {
            init::execute().expect("init should succeed");
            let err = execute("nonexistent", &[]).expect_err("should fail");
            assert!(matches!(err, AppError::RoleNotFound(_)));
        });
    }

    #[test]
    #[serial]
    fn assign_finds_role_by_exact_name() {
        with_temp_cwd(|_dir| {
            init::execute().expect("init should succeed");
            // This will fail on CI without clipboard, but the role lookup should work
            let result = execute("taxonomy", &[]);
            // If clipboard works, it succeeds; if not, it's a ClipboardError
            match result {
                Ok(role_id) => assert_eq!(role_id, "taxonomy"),
                Err(AppError::ClipboardError(_)) => {} // OK on headless systems
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        });
    }

    #[test]
    #[serial]
    fn assign_finds_role_by_prefix() {
        with_temp_cwd(|_dir| {
            init::execute().expect("init should succeed");
            let result = execute("tax", &[]);
            match result {
                Ok(role_id) => assert_eq!(role_id, "taxonomy"),
                Err(AppError::ClipboardError(_)) => {}
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        });
    }
}
