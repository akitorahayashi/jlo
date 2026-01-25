//! jo: Deploy and manage .jules/ workspace scaffolding for organizational memory.

mod bundle;
mod commands;
pub mod error;
mod workspace;

use commands::{init, role, session, status, update};
use error::AppError;
use std::path::PathBuf;

/// Initialize a new `.jules/` workspace in the current directory.
pub fn init(force: bool) -> Result<(), AppError> {
    let options = init::InitOptions { force };
    init::execute(&options)?;
    println!("✅ Initialized .jules/ workspace");
    Ok(())
}

/// Update jo-managed files under `.jules/.jo/`.
pub fn update(force: bool) -> Result<(), AppError> {
    let options = update::UpdateOptions { force };
    let result = update::execute(&options)?;

    match result.previous_version {
        Some(prev) if prev != result.new_version => {
            println!("✅ Updated from {} to {}", prev, result.new_version);
        }
        Some(_) => {
            println!("✅ Refreshed jo-managed files (version {})", result.new_version);
        }
        None => {
            println!("✅ Deployed jo-managed files (version {})", result.new_version);
        }
    }

    Ok(())
}

/// Print status information about the workspace.
pub fn status() -> Result<(), AppError> {
    let result = status::execute()?;

    println!("jo version: {}", result.installed_version);

    if !result.workspace_exists {
        println!("⚠️  No .jules/ workspace in current directory");
        println!("   Run 'jo init' to create one");
        return Ok(());
    }

    if let Some(ref version) = result.workspace_version {
        println!("Workspace version: {}", version);
    } else {
        println!("Workspace version: (unknown)");
    }

    if result.update_available {
        println!("📦 Update available - run 'jo update' to apply");
    } else {
        println!("✅ Workspace is up to date");
    }

    if !result.modified_files.is_empty() {
        println!("\n⚠️  Modified jo-managed files:");
        for file in &result.modified_files {
            println!("   - {}", file);
        }
        println!("\n   Run 'jo update --force' to restore them");
    }

    Ok(())
}

/// Create a role workspace under `.jules/roles/`.
pub fn role(role_id: &str) -> Result<(), AppError> {
    let options = role::RoleOptions { role_id };
    role::execute(&options)?;
    println!("✅ Created role '{}'", role_id);
    Ok(())
}

/// Create a new session file for a role.
pub fn session(role_id: &str, slug: Option<&str>) -> Result<PathBuf, AppError> {
    let options = session::SessionOptions { role_id, slug };
    let path = session::execute(&options)?;
    println!("✅ Created session: {}", path.display());
    Ok(path)
}
