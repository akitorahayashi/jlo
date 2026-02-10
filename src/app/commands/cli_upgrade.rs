//! Upgrade the jlo CLI binary from the upstream Git repository.
//!
//! This command compares the current binary version with the latest semver tag
//! from the configured upstream repository. If upstream is newer, it runs
//! `cargo install --git ... --tag ... --force jlo`.

use std::cmp::Ordering;
use std::process::{Command, Output};

use crate::domain::AppError;

const JLO_GIT_HTTP_URL: &str = "https://github.com/akitorahayashi/jlo.git";

/// Result of a CLI upgrade check/execution.
#[derive(Debug, Clone)]
pub struct CliUpgradeResult {
    /// Current binary version (Cargo package version).
    pub current_version: String,
    /// Latest semver tag found upstream (e.g. `v9.4.1`).
    pub latest_tag: String,
    /// Whether an upgrade was applied.
    pub upgraded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VersionTriplet {
    major: u64,
    minor: u64,
    patch: u64,
}

impl VersionTriplet {
    fn parse(value: &str) -> Option<Self> {
        let normalized = value.trim().trim_start_matches('v');
        let core = normalized.split_once('-').map_or(normalized, |(head, _)| head);
        let mut parts = core.split('.');

        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;

        if parts.next().is_some() {
            return None;
        }

        Some(Self { major, minor, patch })
    }

    fn cmp(self, other: Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

/// Execute CLI upgrade check and apply update when needed.
pub fn execute() -> Result<CliUpgradeResult, AppError> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let current = VersionTriplet::parse(&current_version).ok_or_else(|| {
        AppError::Validation(format!(
            "Current binary version '{}' is not valid semver.",
            current_version
        ))
    })?;

    let tags_output = run_command_capture(
        "git",
        &["ls-remote", "--tags", "--refs", JLO_GIT_HTTP_URL],
        "git ls-remote",
    )?;

    let latest_tag = latest_release_tag(&tags_output).ok_or_else(|| {
        AppError::Validation(format!("No semver release tags found in '{}'.", JLO_GIT_HTTP_URL))
    })?;
    let latest = VersionTriplet::parse(&latest_tag).ok_or_else(|| {
        AppError::Validation(format!("Latest tag '{}' is not valid semver.", latest_tag))
    })?;

    if latest.cmp(current) != Ordering::Greater {
        return Ok(CliUpgradeResult { current_version, latest_tag, upgraded: false });
    }

    run_command_status(
        "cargo",
        &["install", "--git", JLO_GIT_HTTP_URL, "--tag", &latest_tag, "--force", "jlo"],
        "cargo install",
    )?;

    Ok(CliUpgradeResult { current_version, latest_tag, upgraded: true })
}

fn latest_release_tag(ls_remote_output: &str) -> Option<String> {
    ls_remote_output
        .lines()
        .filter_map(extract_tag_ref)
        .filter_map(|tag| VersionTriplet::parse(tag).map(|version| (version, tag.to_string())))
        .max_by(|(left, _), (right, _)| left.cmp(*right))
        .map(|(_, tag)| tag)
}

fn extract_tag_ref(line: &str) -> Option<&str> {
    line.split_whitespace().nth(1)?.strip_prefix("refs/tags/")
}

fn run_command_capture(program: &str, args: &[&str], tool_name: &str) -> Result<String, AppError> {
    let output = run_command(program, args, tool_name)?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run_command_status(program: &str, args: &[&str], tool_name: &str) -> Result<(), AppError> {
    run_command(program, args, tool_name)?;
    Ok(())
}

fn run_command(program: &str, args: &[&str], tool_name: &str) -> Result<Output, AppError> {
    let output = Command::new(program).args(args).output().map_err(|err| {
        AppError::ExternalToolError { tool: tool_name.to_string(), error: err.to_string() }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::ExternalToolError {
            tool: tool_name.to_string(),
            error: format!("command failed: {} {}", program, args.join(" ")).to_string()
                + &format!("\nstderr:\n{}", stderr.trim()),
        });
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_triplet_parses_with_or_without_v_prefix() {
        assert_eq!(
            VersionTriplet::parse("v9.4.1"),
            Some(VersionTriplet { major: 9, minor: 4, patch: 1 })
        );
        assert_eq!(
            VersionTriplet::parse("9.4.1"),
            Some(VersionTriplet { major: 9, minor: 4, patch: 1 })
        );
    }

    #[test]
    fn version_triplet_rejects_invalid_shapes() {
        assert_eq!(VersionTriplet::parse("9.4"), None);
        assert_eq!(VersionTriplet::parse("v9.4.1.0"), None);
        assert_eq!(VersionTriplet::parse("abc"), None);
    }

    #[test]
    fn latest_release_tag_picks_highest_semver() {
        let input = r#"
deadbeef	refs/tags/v9.2.2
deadbeef	refs/tags/v9.3.0
deadbeef	refs/tags/v9.10.0
"#;
        assert_eq!(latest_release_tag(input), Some("v9.10.0".to_string()));
    }

    #[test]
    fn latest_release_tag_ignores_non_semver_tags() {
        let input = r#"
deadbeef	refs/tags/release
deadbeef	refs/tags/nightly
deadbeef	refs/tags/v9.3.0
"#;
        assert_eq!(latest_release_tag(input), Some("v9.3.0".to_string()));
    }
}
