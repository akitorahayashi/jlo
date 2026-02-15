use std::path::{Path, PathBuf};

/// `.jules/exchange/`
pub fn exchange_dir(jules_path: &Path) -> PathBuf {
    jules_path.join("exchange")
}

/// `.jules/exchange/changes.yml`
pub fn exchange_changes(jules_path: &Path) -> PathBuf {
    exchange_dir(jules_path).join("changes.yml")
}
