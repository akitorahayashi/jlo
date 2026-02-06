//! Deinit command implementation.

use crate::domain::AppError;

pub fn run_deinit() -> Result<(), AppError> {
    let outcome = crate::api::deinit()?;

    if outcome.deleted_branch {
        println!("✅ Deleted local 'jules' branch");
    } else {
        println!("ℹ️ Local 'jules' branch not found");
    }

    if outcome.deleted_files.is_empty() && outcome.deleted_action_dirs.is_empty() {
        println!("ℹ️ No workflow kit files found to remove");
    } else {
        if !outcome.deleted_files.is_empty() {
            println!("✅ Removed {} workflow kit file(s)", outcome.deleted_files.len());
        }
        if !outcome.deleted_action_dirs.is_empty() {
            println!(
                "✅ Removed {} workflow action directory(ies)",
                outcome.deleted_action_dirs.len()
            );
        }
    }

    println!("⚠️ Remove JULES_API_KEY and JULES_API_SECRET from GitHub repository settings.");
    Ok(())
}
