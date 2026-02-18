use serde::{Deserialize, Serialize};

/// Innovator perspective definition.
///
/// Originally defined as `PerspectiveData` in `publish_proposals.rs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InnovatorPerspective {
    #[serde(default)]
    pub recent_proposals: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObserverPerspective {
    pub schema_version: u32,
    pub observer: String,
    pub updated_at: String,
    #[serde(default)]
    pub goals: Vec<String>,
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    #[serde(default)]
    pub log: Vec<LogEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    pub at: String,
    pub summary: String,
}
