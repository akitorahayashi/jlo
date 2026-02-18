use serde::{Deserialize, Serialize};

/// Innovator proposal definition.
///
/// Originally defined as `ProposalData` in `publish_proposals.rs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proposal {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub problem: String,
    #[serde(default)]
    pub introduction: String,
    #[serde(default)]
    pub importance: String,
    #[serde(default)]
    pub impact_surface: Vec<String>,
    #[serde(default)]
    pub implementation_cost: String,
    #[serde(default)]
    pub consistency_risks: Vec<String>,
    #[serde(default)]
    pub verification_signals: Vec<String>,
}
