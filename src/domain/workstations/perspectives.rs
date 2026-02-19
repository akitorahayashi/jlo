use serde::{Deserialize, Serialize};

/// Placeholder for date values in scaffold templates and validation.
pub const DATETIME_PLACEHOLDER: &str = "YYYY-MM-DD";

/// Innovator perspective definition.
///
/// Originally defined as `Perspective` in `publish_proposals.rs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InnovatorPerspective {
    pub schema_version: u32,
    pub role: String,
    pub focus: String,
    #[serde(default)]
    pub recent_proposals: Vec<String>,
}

/// Observer perspective definition.
///
/// Currently used for domain modeling and serialization compliance.
/// Validation logic is handled via `serde_yaml::Mapping` in `doctor/schema.rs`
/// to allow for partial validation and better error reporting.
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
