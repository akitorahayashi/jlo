//! Jules API client implementation using reqwest.

use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::domain::{AppError, JulesApiConfig};
use crate::ports::{JulesClient, SessionRequest, SessionResponse};

const X_GOOG_API_KEY: &str = "X-Goog-Api-Key";

/// HTTP client for Jules API.
#[derive(Clone)]
pub struct HttpJulesClient {
    api_key: String,
    api_url: Url,
    max_retries: u32,
    retry_delay_ms: u64,
    client: Client,
}

impl std::fmt::Debug for HttpJulesClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpJulesClient")
            .field("api_url", &self.api_url)
            .field("max_retries", &self.max_retries)
            .field("retry_delay_ms", &self.retry_delay_ms)
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
            .map_err(|e| AppError::Configuration(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            api_key,
            api_url: config.api_url.clone(),
            max_retries: config.max_retries,
            retry_delay_ms: config.retry_delay_ms,
            client,
        })
    }

    /// Create from environment variable with default configuration.
    #[allow(dead_code)]
    pub fn from_env() -> Result<Self, AppError> {
        let api_key = std::env::var("JULES_API_KEY").map_err(|_| {
            AppError::Configuration("JULES_API_KEY environment variable not set".into())
        })?;

        Self::new(api_key, &JulesApiConfig::default())
    }

    /// Create from environment variable with custom configuration.
    pub fn from_env_with_config(config: &JulesApiConfig) -> Result<Self, AppError> {
        let api_key = std::env::var("JULES_API_KEY").map_err(|_| {
            AppError::Configuration("JULES_API_KEY environment variable not set".into())
        })?;

        Self::new(api_key, config)
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
    #[serde(default)]
    #[allow(dead_code)]
    error: Option<String>,
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

        let mut last_error = None;
        let max_attempts = self.max_retries.max(1); // Ensure at least one attempt

        for attempt in 0..max_attempts {
            if attempt > 0 {
                // Exponential backoff: base * 2^(attempt-1)
                let delay = self.retry_delay_ms * 2_u64.pow(attempt.saturating_sub(1));
                std::thread::sleep(Duration::from_millis(delay));
                println!("Retrying... (attempt {}/{})", attempt + 1, max_attempts);
            }

            match self.send_request(&api_request) {
                Ok(response) => return Ok(response),
                Err(e) => {
                    if Self::is_retryable(&e) {
                        last_error = Some(e);
                        continue;
                    }
                    return Err(e);
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| AppError::Configuration("Request failed after all retries".into())))
    }
}

impl HttpJulesClient {
    fn send_request(&self, request: &ApiRequest) -> Result<SessionResponse, AppError> {
        let response = self
            .client
            .post(self.api_url.clone())
            .header(X_GOOG_API_KEY, &self.api_key)
            .header(CONTENT_TYPE, "application/json")
            .json(request)
            .send()
            .map_err(|e| AppError::Configuration(format!("HTTP request failed: {}", e)))?;

        let status = response.status();

        if status.is_success() {
            let api_response: ApiResponse = response
                .json()
                .map_err(|e| AppError::Configuration(format!("Failed to parse response: {}", e)))?;

            let session_id = api_response
                .session_id
                .or(api_response.id)
                .ok_or_else(|| AppError::Configuration("No session ID in response".into()))?;

            Ok(SessionResponse {
                session_id,
                status: api_response.status.unwrap_or_else(|| "created".to_string()),
            })
        } else if status.as_u16() == 429 {
            Err(AppError::Configuration("Rate limited (429)".into()))
        } else if status.is_server_error() {
            Err(AppError::Configuration(format!("Server error ({})", status.as_u16())))
        } else {
            let error_text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
            Err(AppError::Configuration(format!("API error ({}): {}", status.as_u16(), error_text)))
        }
    }

    fn is_retryable(error: &AppError) -> bool {
        match error {
            AppError::Configuration(msg) => {
                msg.contains("429") || msg.contains("Server error") || msg.contains("timeout")
            }
            _ => false,
        }
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
    fn create_session_retries_on_500() {
        let mut server = mockito::Server::new();
        let mock = server.mock("POST", "/").with_status(500).expect(3).create();

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
    fn create_session_retries_on_429() {
        let mut server = mockito::Server::new();
        let mock = server.mock("POST", "/").with_status(429).expect(3).create();

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
}
