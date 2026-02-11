use std::fmt;
use std::str::FromStr;

use crate::domain::AppError;

/// Runner mode for workflow scaffolds.
///
/// `"remote"` maps to GitHub-hosted runners (`ubuntu-latest`).
/// Any other value is passed through as the `runs-on` label,
/// enabling custom self-hosted runner configurations
/// (e.g. `self-hosted`, `my-mac-mini`, `[self-hosted, macOS, arm64]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunnerMode(String);

impl WorkflowRunnerMode {
    /// Well-known shortcut for GitHub-hosted runners.
    pub const REMOTE: &str = "remote";
    /// Well-known shortcut for the generic self-hosted label.
    pub const SELF_HOSTED: &str = "self-hosted";

    /// The config value as written in `.jlo/config.toml`.
    pub fn label(&self) -> &str {
        &self.0
    }

    /// The `runs-on` value rendered into workflow YAML.
    ///
    /// `"remote"` becomes `ubuntu-latest`; everything else is passed through verbatim.
    pub fn runner_label(&self) -> &str {
        if self.0 == Self::REMOTE {
            "ubuntu-latest"
        } else {
            &self.0
        }
    }

    pub fn remote() -> Self {
        Self(Self::REMOTE.to_string())
    }

    pub fn self_hosted() -> Self {
        Self(Self::SELF_HOSTED.to_string())
    }
}

impl FromStr for WorkflowRunnerMode {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation(
                "Runner mode must not be empty.".into(),
            ));
        }
        Ok(Self(trimmed.to_string()))
    }
}

impl fmt::Display for WorkflowRunnerMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
