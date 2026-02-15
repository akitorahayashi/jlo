use std::path::{Path, PathBuf};

use crate::domain::exchange;

/// `.jules/exchange/requirements/`
pub fn requirements_dir(jules_path: &Path) -> PathBuf {
    exchange::paths::exchange_dir(jules_path).join("requirements")
}
