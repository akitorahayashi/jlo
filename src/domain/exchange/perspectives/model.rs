use serde::{Deserialize, Serialize};

/// Innovator perspective definition.
///
/// Originally defined as `PerspectiveData` in `publish_proposals.rs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Perspective {
    #[serde(default)]
    pub recent_proposals: Vec<String>,
}
