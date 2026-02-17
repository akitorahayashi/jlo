//! Shared layer policy functions.
//!
//! Policy decisions used by both `jlo run` and `jlo workflow run` command families.
//! Each function encapsulates one layer-specific guard that determines whether
//! dispatch should proceed.

use std::path::Path;

use crate::domain::AppError;

/// Check whether `.jules/exchange/events/pending/` contains at least one `.yml` file.
///
/// Used by both `jlo run decider` and `jlo workflow run decider` to decide
/// whether the decider layer should dispatch.
pub fn has_pending_events(jules_path: &Path) -> Result<bool, AppError> {
    let pending_dir =
        crate::domain::exchange::paths::exchange_dir(jules_path).join("events/pending");
    if !pending_dir.exists() {
        return Ok(false);
    }
    let entries = std::fs::read_dir(&pending_dir)?;
    for entry in entries {
        let entry = entry?;
        if entry.path().is_file() && entry.path().extension().is_some_and(|ext| ext == "yml") {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn no_pending_dir_returns_false() {
        let dir = tempdir().unwrap();
        let jules = dir.path().join(".jules");
        assert!(!has_pending_events(&jules).unwrap());
    }

    #[test]
    fn empty_pending_dir_returns_false() {
        let dir = tempdir().unwrap();
        let pending = dir.path().join(".jules/exchange/events/pending");
        std::fs::create_dir_all(&pending).unwrap();
        assert!(!has_pending_events(&dir.path().join(".jules")).unwrap());
    }

    #[test]
    fn pending_yml_returns_true() {
        let dir = tempdir().unwrap();
        let pending = dir.path().join(".jules/exchange/events/pending");
        std::fs::create_dir_all(&pending).unwrap();
        std::fs::write(pending.join("event1.yml"), "id: e1").unwrap();
        assert!(has_pending_events(&dir.path().join(".jules")).unwrap());
    }

    #[test]
    fn pending_non_yml_ignored() {
        let dir = tempdir().unwrap();
        let pending = dir.path().join(".jules/exchange/events/pending");
        std::fs::create_dir_all(&pending).unwrap();
        std::fs::write(pending.join("README.md"), "info").unwrap();
        assert!(!has_pending_events(&dir.path().join(".jules")).unwrap());
    }
}
