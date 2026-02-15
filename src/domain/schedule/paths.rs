use std::path::{Path, PathBuf};

use crate::domain::workstations;

/// The scheduled execution file name.
pub const SCHEDULED_FILENAME: &str = "scheduled.toml";

/// `.jlo/scheduled.toml`
pub fn schedule(root: &Path) -> PathBuf {
    workstations::paths::jlo_dir(root).join(SCHEDULED_FILENAME)
}

/// `.jlo/scheduled.toml` relative string.
pub fn schedule_relative() -> &'static str {
    ".jlo/scheduled.toml"
}
