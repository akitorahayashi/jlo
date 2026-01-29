//! Jules API client port definition.

use crate::domain::AppError;

/// Request to create a Jules session.
#[derive(Debug, Clone)]
pub struct SessionRequest {
    /// The prompt to send to Jules.
    pub prompt: String,
    /// Source identifier (e.g., "sources/github/owner/repo").
    pub source: String,
    /// Branch for Jules to start from.
    pub starting_branch: String,
    /// Whether plan approval is required.
    pub require_plan_approval: bool,
    /// Automation mode for PR creation.
    pub automation_mode: AutomationMode,
}

/// Automation mode for Jules session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutomationMode {
    /// Automatically create a PR when complete.
    #[default]
    AutoCreatePr,
    /// Create a draft PR.
    DraftPr,
    /// No automatic PR creation.
    None,
}

impl AutomationMode {
    /// Convert to API string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            AutomationMode::AutoCreatePr => "AUTO_CREATE_PR",
            AutomationMode::DraftPr => "DRAFT_PR",
            AutomationMode::None => "NONE",
        }
    }
}

/// Response from Jules session creation.
#[derive(Debug, Clone)]
pub struct SessionResponse {
    /// Session ID.
    pub session_id: String,
    /// Status of the session.
    pub status: String,
}

/// Port for Jules API operations.
pub trait JulesClient {
    /// Create a new Jules session.
    fn create_session(&self, request: SessionRequest) -> Result<SessionResponse, AppError>;
}

/// Mock client for testing without API calls.
#[derive(Debug, Clone, Default)]
pub struct MockJulesClient;

impl JulesClient for MockJulesClient {
    fn create_session(&self, request: SessionRequest) -> Result<SessionResponse, AppError> {
        println!("=== MOCK MODE ===");
        println!("Would invoke Jules with:");
        println!("  Source: {}", request.source);
        println!("  Starting branch: {}", request.starting_branch);
        println!("  Automation: {}", request.automation_mode.as_str());
        println!("  Prompt length: {} chars", request.prompt.len());

        Ok(SessionResponse {
            session_id: format!("mock-{}", chrono::Utc::now().timestamp()),
            status: "mock".to_string(),
        })
    }
}
