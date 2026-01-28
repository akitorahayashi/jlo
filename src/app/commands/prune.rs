use std::process::Command;

use crate::app::AppContext;
use crate::domain::AppError;
use crate::ports::{ClipboardWriter, RoleTemplateStore, WorkspaceStore};

/// Execute the prune command.
pub fn execute<W, R, C>(ctx: &AppContext<W, R, C>, days: u32, dry_run: bool) -> Result<(), AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
    C: ClipboardWriter,
{
    if !ctx.workspace().exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // Discover role names from .jules/roles/*/
    let roles = ctx.workspace().discover_roles()?;
    if roles.is_empty() {
        println!("No roles found in .jules/roles/");
        return Ok(());
    }

    // Build a set of known role IDs for efficient lookup
    let known_roles: std::collections::HashSet<_> = roles.into_iter().map(|r| r.id).collect();

    // Get remote branches
    let output = Command::new("git")
        .args(["branch", "-r", "--format=%(refname:short)"])
        .output()
        .map_err(|e| AppError::config_error(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::config_error(format!("git branch failed: {}", stderr)));
    }

    let branches = String::from_utf8_lossy(&output.stdout);
    let now = chrono::Utc::now();
    let cutoff = now - chrono::Duration::days(days as i64);

    let mut to_delete: Vec<String> = Vec::new();

    for line in branches.lines() {
        let branch = line.trim();
        if !branch.starts_with("origin/jules/") {
            continue;
        }

        let short_name = branch.strip_prefix("origin/").unwrap_or(branch);

        // Parse role and timestamp from branch name: jules/<layer>-<role>-YYYYMMDD-HHMM-<id>
        if let Some((role_id, timestamp)) = parse_branch_info(short_name)
            && known_roles.contains(&role_id)
            && timestamp < cutoff
        {
            to_delete.push(branch.to_string());
        }
    }

    if to_delete.is_empty() {
        println!("No branches older than {} days found", days);
        return Ok(());
    }

    if dry_run {
        println!("Would delete {} branches:", to_delete.len());
        for branch in &to_delete {
            println!("  {}", branch);
        }
    } else {
        println!("Deleting {} branches...", to_delete.len());
        for branch in &to_delete {
            let remote_branch = branch.strip_prefix("origin/").unwrap_or(branch);
            let status = Command::new("git")
                .args(["push", "origin", "--delete", remote_branch])
                .status()
                .map_err(|e| AppError::config_error(format!("Failed to run git push: {}", e)))?;

            if status.success() {
                println!("  Deleted: {}", remote_branch);
            } else {
                eprintln!("  Failed to delete: {}", remote_branch);
            }
        }
        println!("Done.");
    }

    Ok(())
}

/// Parse role ID and timestamp from branch name format: jules/<layer>-<role>-YYYYMMDD-HHMM-<id>
fn parse_branch_info(branch: &str) -> Option<(String, chrono::DateTime<chrono::Utc>)> {
    // Remove "jules/" prefix
    let name = branch.strip_prefix("jules/")?;

    // Format: <layer>-<role>-YYYYMMDD-HHMM-<id>
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() < 4 {
        return None;
    }

    // Find the date part (8 digits)
    let date_idx =
        parts.iter().position(|p| p.len() == 8 && p.chars().all(|c| c.is_ascii_digit()))?;

    // Ensure there's at least layer-role before date and time after
    if date_idx < 2 || date_idx + 1 >= parts.len() {
        return None;
    }

    // Role is the second part (after layer)
    let role_id = parts[1].to_string();
    let date_str = parts[date_idx];
    let time_str = parts[date_idx + 1];

    if time_str.len() != 4 || !time_str.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let year: i32 = date_str[0..4].parse().ok()?;
    let month: u32 = date_str[4..6].parse().ok()?;
    let day: u32 = date_str[6..8].parse().ok()?;
    let hour: u32 = time_str[0..2].parse().ok()?;
    let minute: u32 = time_str[2..4].parse().ok()?;

    let dt = chrono::NaiveDate::from_ymd_opt(year, month, day)
        .and_then(|d| d.and_hms_opt(hour, minute, 0))
        .map(|dt| dt.and_utc())?;

    Some((role_id, dt))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_branch_info_valid() {
        let result = parse_branch_info("jules/observer-taxonomy-20260128-1345-a1b2");
        assert!(result.is_some());
        let (role, ts) = result.unwrap();
        assert_eq!(role, "taxonomy");
        assert_eq!(ts.format("%Y%m%d-%H%M").to_string(), "20260128-1345");
    }

    #[test]
    fn parse_branch_info_merger() {
        let result = parse_branch_info("jules/merger-consolidator-20260128-1415-e5f6");
        assert!(result.is_some());
        let (role, _) = result.unwrap();
        assert_eq!(role, "consolidator");
    }

    #[test]
    fn parse_branch_info_invalid() {
        assert!(parse_branch_info("jules/observer-taxonomy").is_none());
        assert!(parse_branch_info("main").is_none());
        assert!(parse_branch_info("feature/something").is_none());
        // Too few parts before date
        assert!(parse_branch_info("jules/taxonomy-20260128-1345-a1b2").is_none());
    }
}
