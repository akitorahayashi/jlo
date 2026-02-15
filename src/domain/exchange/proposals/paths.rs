use std::path::{Path, PathBuf};

use crate::domain::exchange;

/// `.jules/exchange/proposals/`
pub fn proposals_dir(jules_path: &Path) -> PathBuf {
    exchange::paths::exchange_dir(jules_path).join("proposals")
}

/// `.jules/exchange/proposals/<persona>-<slug>.yml`
pub fn proposal_file(jules_path: &Path, persona: &str, slug: &str) -> PathBuf {
    proposals_dir(jules_path).join(format!("{}-{}.yml", persona, slug))
}
