use serde::Deserialize;

/// Header information for an issue.
#[derive(Debug, Clone, Deserialize)]
pub struct IssueHeader {
    /// Whether the issue requires deep analysis (planning).
    pub requires_deep_analysis: bool,
}
