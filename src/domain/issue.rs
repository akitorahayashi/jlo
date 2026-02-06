use serde::Deserialize;

/// Header fields for an issue.
#[derive(Debug, Clone, Deserialize)]
pub struct IssueHeader {
    /// Whether the issue requires deep analysis (planner) or implementation (implementer).
    pub requires_deep_analysis: bool,
}
