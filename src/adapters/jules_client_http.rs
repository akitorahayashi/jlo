//! Jules API client implementation using reqwest.

use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderValue, RETRY_AFTER};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::domain::{AppError, JulesApiConfig};
use crate::ports::{JulesClient, SessionRequest, SessionResponse};

const X_GOOG_API_KEY: &str = "X-Goog-Api-Key";
const DEFAULT_STATUS_MESSAGE: &str = "Jules API request failed";

/// HTTP transport for Jules API.
///
/// This client performs a single request per call. Retry behavior is implemented
/// by a dedicated retry wrapper adapter.
#[derive(Clone)]
pub struct HttpJulesClient {
    api_key: String,
    api_url: Url,
    client: Client,
}

impl std::fmt::Debug for HttpJulesClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpJulesClient")
            .field("api_url", &self.api_url)
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}

impl HttpJulesClient {
    /// Create a new HTTP client with the given API key and configuration.
    pub fn new(api_key: String, config: &JulesApiConfig) -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| AppError::JulesApiError {
                message: format!("Failed to create HTTP client: {}", e),
                status: None,
            })?;

        Ok(Self { api_key, api_url: config.api_url.clone(), client })
    }

    /// Create from environment variable with default configuration.
    #[allow(dead_code)]
    pub fn from_env() -> Result<Self, AppError> {
        let api_key = std::env::var("JULES_API_KEY")
            .map_err(|_| AppError::EnvironmentVariableMissing("JULES_API_KEY".into()))?;

        Self::new(api_key, &JulesApiConfig::default())
    }

    /// Create from environment variable with custom configuration.
    pub fn from_env_with_config(config: &JulesApiConfig) -> Result<Self, AppError> {
        let api_key = std::env::var("JULES_API_KEY")
            .map_err(|_| AppError::EnvironmentVariableMissing("JULES_API_KEY".into()))?;

        Self::new(api_key, config)
    }

    fn send_request(&self, request: &ApiRequest) -> Result<SessionResponse, AppError> {
        let response = self
            .client
            .post(self.api_url.clone())
            .header(X_GOOG_API_KEY, &self.api_key)
            .header(CONTENT_TYPE, "application/json")
            .json(request)
            .send()
            .map_err(|e| AppError::JulesApiError {
                message: format!("HTTP request failed: {}", e),
                status: None,
            })?;

        let status = response.status();
        let retry_after_ms = response.headers().get(RETRY_AFTER).and_then(parse_retry_after_ms);
        let body_text = response.text().unwrap_or_default();

        if status.is_success() {
            let api_response: ApiResponse =
                serde_json::from_str(&body_text).map_err(|e| AppError::JulesApiError {
                    message: format!("Failed to parse response: {}", e),
                    status: Some(status.as_u16()),
                })?;

            let session_id = api_response.session_id.or(api_response.id).ok_or_else(|| {
                AppError::JulesApiError {
                    message: "No session ID in response".into(),
                    status: Some(status.as_u16()),
                }
            })?;

            return Ok(SessionResponse {
                session_id,
                status: api_response.status.unwrap_or_else(|| "created".to_string()),
            });
        }

        let mut message = extract_error_message(&body_text).unwrap_or_else(|| {
            if !body_text.trim().is_empty() {
                body_text.clone()
            } else if status.as_u16() == 429 {
                "Rate limited".to_string()
            } else if status.is_server_error() {
                "Server error".to_string()
            } else {
                DEFAULT_STATUS_MESSAGE.to_string()
            }
        });

        if let Some(value) = retry_after_ms {
            message.push_str(&format!(" (retry_after_ms={})", value));
        }

        Err(AppError::JulesApiError { message, status: Some(status.as_u16()) })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiRequest {
    prompt: String,
    source_context: SourceContext,
    require_plan_approval: bool,
    automation_mode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceContext {
    source: String,
    github_repo_context: GithubRepoContext,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GithubRepoContext {
    starting_branch: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiResponse {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

fn extract_error_message(body: &str) -> Option<String> {
    if body.trim().is_empty() {
        return None;
    }

    let parsed = serde_json::from_str::<serde_json::Value>(body).ok()?;

    if let Some(msg) = parsed
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(|message| message.as_str())
    {
        return Some(msg.to_string());
    }

    parsed.get("message").and_then(|message| message.as_str()).map(ToOwned::to_owned)
}

fn parse_retry_after_ms(value: &HeaderValue) -> Option<u64> {
    let raw = value.to_str().ok()?.trim();
    let seconds = raw.parse::<u64>().ok()?;
    Some(seconds.saturating_mul(1000))
}

impl JulesClient for HttpJulesClient {
    fn create_session(&self, request: SessionRequest) -> Result<SessionResponse, AppError> {
        let api_request = ApiRequest {
            prompt: request.prompt,
            source_context: SourceContext {
                source: request.source,
                github_repo_context: GithubRepoContext { starting_branch: request.starting_branch },
            },
            require_plan_approval: request.require_plan_approval,
            automation_mode: request.automation_mode.as_str().to_string(),
        };

        self.send_request(&api_request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::JulesApiConfig;
    use crate::ports::{AutomationMode, SessionRequest};

    #[test]
    fn automation_mode_serializes_correctly() {
        assert_eq!(AutomationMode::AutoCreatePr.as_str(), "AUTO_CREATE_PR");
        assert_eq!(AutomationMode::DraftPr.as_str(), "DRAFT_PR");
        assert_eq!(AutomationMode::None.as_str(), "NONE");
    }

    #[test]
    fn create_session_success() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"sessionId": "test-session", "status": "created"}"#)
            .create();

        let config = JulesApiConfig {
            api_url: Url::parse(&server.url()).unwrap(),
            max_retries: 3,
            retry_delay_ms: 1,
            timeout_secs: 1,
        };

        let client = HttpJulesClient::new("fake-key".to_string(), &config).unwrap();
        let request = SessionRequest {
            prompt: "test".to_string(),
            source: "github".to_string(),
            starting_branch: "main".to_string(),
            require_plan_approval: false,
            automation_mode: AutomationMode::None,
        };

        let result = client.create_session(request);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().session_id, "test-session");
    }

    #[test]
    fn create_session_returns_server_error_on_500() {
        let mut server = mockito::Server::new();
        let mock = server.mock("POST", "/").with_status(500).expect(1).create();

        let config = JulesApiConfig {
            api_url: Url::parse(&server.url()).unwrap(),
            max_retries: 3,
            retry_delay_ms: 1,
            timeout_secs: 1,
        };

        let client = HttpJulesClient::new("fake-key".to_string(), &config).unwrap();
        let request = SessionRequest {
            prompt: "test".to_string(),
            source: "github".to_string(),
            starting_branch: "main".to_string(),
            require_plan_approval: false,
            automation_mode: AutomationMode::None,
        };

        let result = client.create_session(request);
        assert!(result.is_err());
        mock.assert();
    }

    #[test]
    fn create_session_returns_rate_limit_on_429() {
        let mut server = mockito::Server::new();
        let mock = server.mock("POST", "/").with_status(429).expect(1).create();

        let config = JulesApiConfig {
            api_url: Url::parse(&server.url()).unwrap(),
            max_retries: 3,
            retry_delay_ms: 1,
            timeout_secs: 1,
        };

        let client = HttpJulesClient::new("fake-key".to_string(), &config).unwrap();
        let request = SessionRequest {
            prompt: "test".to_string(),
            source: "github".to_string(),
            starting_branch: "main".to_string(),
            require_plan_approval: false,
            automation_mode: AutomationMode::None,
        };

        let result = client.create_session(request);
        assert!(result.is_err());
        mock.assert();
    }

    #[test]
    fn create_session_fails_fast_on_400() {
        let mut server = mockito::Server::new();
        let mock =
            server.mock("POST", "/").with_status(400).with_body("Bad Request").expect(1).create();

        let config = JulesApiConfig {
            api_url: Url::parse(&server.url()).unwrap(),
            max_retries: 3,
            retry_delay_ms: 1,
            timeout_secs: 1,
        };

        let client = HttpJulesClient::new("fake-key".to_string(), &config).unwrap();
        let request = SessionRequest {
            prompt: "test".to_string(),
            source: "github".to_string(),
            starting_branch: "main".to_string(),
            require_plan_approval: false,
            automation_mode: AutomationMode::None,
        };

        let result = client.create_session(request);
        assert!(result.is_err());
        mock.assert();
    }

    #[test]
    fn parses_nested_error_message() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/")
            .with_status(500)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":{"message":"transient upstream failure"}}"#)
            .expect(1)
            .create();

        let config = JulesApiConfig {
            api_url: Url::parse(&server.url()).unwrap(),
            max_retries: 3,
            retry_delay_ms: 1,
            timeout_secs: 1,
        };
        let client = HttpJulesClient::new("fake-key".to_string(), &config).unwrap();

        let request = SessionRequest {
            prompt: "test".to_string(),
            source: "github".to_string(),
            starting_branch: "main".to_string(),
            require_plan_approval: false,
            automation_mode: AutomationMode::None,
        };

        let err = client.create_session(request).unwrap_err();
        match err {
            AppError::JulesApiError { message, status } => {
                assert_eq!(status, Some(500));
                assert_eq!(message, "transient upstream failure");
            }
            other => panic!("unexpected error variant: {}", other),
        }
    }
}
