use std::path::{Path, PathBuf};

use crate::domain::exchange;

/// `.jules/exchange/proposals/`
pub fn proposals_dir(jules_path: &Path) -> PathBuf {
    exchange::paths::exchange_dir(jules_path).join("proposals")
}

/// Normalize role identifier for proposal filename segment.
///
/// Proposal filenames are kebab-case only. Role identifiers may include `_`
/// in config/runtime contexts, so proposal artifacts normalize role segments
/// to lowercase kebab-case.
pub fn proposal_filename_role_segment(role: &str) -> String {
    let mut out = String::with_capacity(role.len());
    for ch in role.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            out.push(ch);
        } else if ch.is_ascii_uppercase() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    out.split('-').filter(|segment| !segment.is_empty()).collect::<Vec<_>>().join("-")
}

/// `.jules/exchange/proposals/<normalized-role>-<slug>.yml`
pub fn proposal_file(jules_path: &Path, role: &str, slug: &str) -> PathBuf {
    proposals_dir(jules_path).join(format!("{}-{}.yml", proposal_filename_role_segment(role), slug))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposal_filename_role_segment_normalizes_to_kebab_case() {
        assert_eq!(proposal_filename_role_segment("leverage_architect"), "leverage-architect");
        assert_eq!(proposal_filename_role_segment("Leverage_Architect"), "leverage-architect");
        assert_eq!(proposal_filename_role_segment("ops--qa"), "ops-qa");
    }
}
