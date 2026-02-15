use std::path::{Path, PathBuf};

use crate::domain::exchange;

/// `.jules/exchange/events/`
pub fn events_dir(jules_path: &Path) -> PathBuf {
    exchange::paths::exchange_dir(jules_path).join("events")
}

/// `.jules/exchange/events/<state>/`
pub fn events_state_dir(jules_path: &Path, state: &str) -> PathBuf {
    events_dir(jules_path).join(state)
}

/// `.jules/exchange/events/pending/`
pub fn events_pending_dir(jules_path: &Path) -> PathBuf {
    events_state_dir(jules_path, "pending")
}

/// `.jules/exchange/events/decided/`
pub fn events_decided_dir(jules_path: &Path) -> PathBuf {
    events_state_dir(jules_path, "decided")
}
