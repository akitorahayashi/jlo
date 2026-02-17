//! Shared layer policy functions.
//!
//! Policy decisions used by both `jlo run` and `jlo workflow run` command families.
//! Each function encapsulates one layer-specific guard that determines whether
//! dispatch should proceed.

use std::path::Path;

use crate::domain::AppError;
use crate::ports::RepositoryFilesystem;

/// Check whether `.jules/exchange/events/pending/` contains at least one `.yml` file.
///
/// Used by both `jlo run decider` and `jlo workflow run decider` to decide
/// whether the decider layer should dispatch.
pub fn has_pending_events(
    store: &impl RepositoryFilesystem,
    jules_path: &Path,
) -> Result<bool, AppError> {
    let pending_dir =
        crate::domain::exchange::paths::exchange_dir(jules_path).join("events/pending");
    let pending_dir_str = pending_dir
        .to_str()
        .ok_or_else(|| AppError::InvalidPath("Pending dir path is not UTF-8".into()))?;

    if !store.is_dir(pending_dir_str) {
        return Ok(false);
    }
    let entries = store.list_dir(pending_dir_str)?;
    for entry in entries {
        let entry_str = entry
            .to_str()
            .ok_or_else(|| AppError::InvalidPath("Entry path is not UTF-8".into()))?;
        if store.file_exists(entry_str)
            && !store.is_dir(entry_str)
            && entry.extension().is_some_and(|ext| ext == "yml")
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TestStore;

    #[test]
    fn no_pending_dir_returns_false() {
        let store = TestStore::new();
        let jules = Path::new(".jules");
        assert!(!has_pending_events(&store, jules).unwrap());
    }

    #[test]
    fn empty_pending_dir_returns_false() {
        let store = TestStore::new();
        store.create_dir_all(".jules/exchange/events/pending").unwrap();
        let jules = Path::new(".jules");
        assert!(!has_pending_events(&store, jules).unwrap());
    }

    #[test]
    fn pending_yml_returns_true() {
        let store = TestStore::new();
        store.create_dir_all(".jules/exchange/events/pending").unwrap();
        store.write_file(".jules/exchange/events/pending/event1.yml", "id: e1").unwrap();
        let jules = Path::new(".jules");
        assert!(has_pending_events(&store, jules).unwrap());
    }

    #[test]
    fn pending_non_yml_ignored() {
        let store = TestStore::new();
        store.create_dir_all(".jules/exchange/events/pending").unwrap();
        store.write_file(".jules/exchange/events/pending/README.md", "info").unwrap();
        let jules = Path::new(".jules");
        assert!(!has_pending_events(&store, jules).unwrap());
    }
}
