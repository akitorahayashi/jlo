//! jo: Deploy and manage .jules/ workspace scaffolding for organizational memory.

mod commands;
pub mod error;
mod scaffold;
mod workspace;

use commands::{init, role, update};
use error::AppError;

/// Initialize a new `.jules/` workspace in the current directory.
pub fn init(force: bool) -> Result<(), AppError> {
    let options = init::InitOptions { force };
    init::execute(&options)?;
    println!("✅ Initialized .jules/ workspace");
    Ok(())
}

/// Update jo-managed files and structural scaffolding in `.jules/`.
pub fn update() -> Result<(), AppError> {
    let result = update::execute()?;

    if !result.updated {
        println!("✅ Workspace already up to date (version {})", result.new_version);
        return Ok(());
    }

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

/// Interactive role selection and scheduler prompt generation.
pub fn role_interactive() -> Result<String, AppError> {
    role::execute()
}
