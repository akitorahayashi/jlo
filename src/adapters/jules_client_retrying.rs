//! Retry wrapper for Jules API client operations.

use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::domain::{AppError, JulesApiConfig};
use crate::ports::{JulesClient, SessionRequest, SessionResponse};

const DEFAULT_MAX_DELAY_MS: u64 = 30_000;
const RETRY_AFTER_TOKEN: &str = "retry_after_ms=";
const MAX_LOG_ERROR_CHARS: usize = 512;

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    max_attempts: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
}

impl RetryPolicy {
    pub fn from_config(config: &JulesApiConfig) -> Self {
        Self {
            max_attempts: config.max_retries.max(1),
            base_delay_ms: config.retry_delay_ms.max(1),
            max_delay_ms: DEFAULT_MAX_DELAY_MS.max(config.retry_delay_ms),
        }
    }

    fn delay_for_retry(&self, failed_attempt: u32, error: &AppError) -> Duration {
        if let Some(retry_after_ms) = extract_retry_after_ms(error) {
            return Duration::from_millis(retry_after_ms.min(self.max_delay_ms));
        }

        // attempt=1 -> base, attempt=2 -> base*2, attempt=3 -> base*4, capped.
        let exponent = failed_attempt.saturating_sub(1).min(6);
        let multiplier = 1_u64 << exponent;
        let backoff_ms = self.base_delay_ms.saturating_mul(multiplier).min(self.max_delay_ms);
        let jitter_ms = compute_jitter_ms(backoff_ms);
        Duration::from_millis(backoff_ms.saturating_add(jitter_ms).min(self.max_delay_ms))
    }
}

pub struct RetryingJulesClient {
    inner: Box<dyn JulesClient>,
    policy: RetryPolicy,
}

impl RetryingJulesClient {
    pub fn new(inner: Box<dyn JulesClient>, policy: RetryPolicy) -> Self {
        Self { inner, policy }
    }
}

impl JulesClient for RetryingJulesClient {
    fn create_session(&self, request: SessionRequest) -> Result<SessionResponse, AppError> {
        let mut last_error: Option<AppError> = None;

        for attempt in 1..=self.policy.max_attempts {
            match self.inner.create_session(request.clone()) {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let retryable = is_retryable_error(&error);
                    let last_attempt = attempt == self.policy.max_attempts;

                    if !retryable || last_attempt {
                        return Err(error);
                    }

                    let delay = self.policy.delay_for_retry(attempt, &error);
                    let log_error = format_error_for_log(&error);
                    eprintln!(
                        "Jules create_session failed (attempt {}/{}): {}. Retrying in {} ms.",
                        attempt,
                        self.policy.max_attempts,
                        log_error,
                        delay.as_millis()
                    );
                    last_error = Some(error);
                    thread::sleep(delay);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AppError::JulesApiError {
            message: "Jules request failed after retries".to_string(),
            status: None,
        }))
    }
}

fn is_retryable_error(error: &AppError) -> bool {
    match error {
        AppError::JulesApiError { message, status } => {
            if status.is_some_and(|code| code == 429 || code == 408 || code >= 500) {
                return true;
            }

            let lower = message.to_ascii_lowercase();
            lower.contains("timeout")
                || lower.contains("timed out")
                || lower.contains("connect")
                || lower.contains("connection")
                || lower.contains("temporary")
                || lower.contains(".mx/error.md")
        }
        _ => false,
    }
}

fn extract_retry_after_ms(error: &AppError) -> Option<u64> {
    let message = match error {
        AppError::JulesApiError { message, .. } => message,
        _ => return None,
    };

    let start = message.find(RETRY_AFTER_TOKEN)? + RETRY_AFTER_TOKEN.len();
    let tail = &message[start..];
    let digits: String = tail.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u64>().ok()
}

fn compute_jitter_ms(backoff_ms: u64) -> u64 {
    if backoff_ms <= 1 {
        return 0;
    }

    let jitter_cap = backoff_ms / 4; // 25% jitter upper bound
    if jitter_cap == 0 {
        return 0;
    }

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as u64)
        .unwrap_or(0);

    nanos % jitter_cap
}

fn format_error_for_log(error: &AppError) -> String {
    match error {
        AppError::JulesApiError { message, status } => {
            let sanitized = sanitize_and_truncate_for_log(message);
            match status {
                Some(code) => format!("JulesApiError(status={}): {}", code, sanitized),
                None => format!("JulesApiError: {}", sanitized),
            }
        }
        _ => sanitize_and_truncate_for_log(&error.to_string()),
    }
}

fn sanitize_and_truncate_for_log(input: &str) -> String {
    let mut output = String::new();

    for (count, ch) in input.chars().enumerate() {
        if count >= MAX_LOG_ERROR_CHARS {
            break;
        }
        output.push(if ch.is_control() { ' ' } else { ch });
    }

    let mut compact = output.split_whitespace().collect::<Vec<_>>().join(" ");
    if input.chars().count() > MAX_LOG_ERROR_CHARS {
        compact.push_str(" [truncated]");
    }
    compact.trim().to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    struct SequenceClient {
        attempts: AtomicUsize,
        responses: std::sync::Mutex<Vec<Result<SessionResponse, AppError>>>,
    }

    impl SequenceClient {
        fn new(responses: Vec<Result<SessionResponse, AppError>>) -> Self {
            Self { attempts: AtomicUsize::new(0), responses: std::sync::Mutex::new(responses) }
        }
    }

    impl JulesClient for SequenceClient {
        fn create_session(&self, _request: SessionRequest) -> Result<SessionResponse, AppError> {
            self.attempts.fetch_add(1, Ordering::SeqCst);
            let mut guard = self.responses.lock().expect("responses lock poisoned");
            if guard.is_empty() {
                return Err(AppError::JulesApiError {
                    message: "test: unexpected extra call".to_string(),
                    status: Some(500),
                });
            }
            guard.remove(0)
        }
    }

    fn test_request() -> SessionRequest {
        SessionRequest {
            prompt: "test prompt".to_string(),
            source: "sources/github/owner/repo".to_string(),
            starting_branch: "main".to_string(),
            require_plan_approval: false,
            automation_mode: crate::ports::AutomationMode::None,
        }
    }

    fn policy(max_attempts: u32) -> RetryPolicy {
        RetryPolicy { max_attempts, base_delay_ms: 1, max_delay_ms: 2 }
    }

    #[test]
    fn retries_transient_failures_and_succeeds() {
        let inner = SequenceClient::new(vec![
            Err(AppError::JulesApiError { message: "server error".to_string(), status: Some(500) }),
            Err(AppError::JulesApiError { message: "rate limited".to_string(), status: Some(429) }),
            Ok(SessionResponse {
                session_id: "session-123".to_string(),
                status: "created".to_string(),
            }),
        ]);
        let client = RetryingJulesClient::new(Box::new(inner), policy(3));

        let result = client.create_session(test_request());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().session_id, "session-123");
    }

    #[test]
    fn does_not_retry_on_non_retryable_error() {
        let inner = SequenceClient::new(vec![Err(AppError::JulesApiError {
            message: "invalid request".to_string(),
            status: Some(400),
        })]);
        let client = RetryingJulesClient::new(Box::new(inner), policy(3));

        let result = client.create_session(test_request());
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::JulesApiError { status, .. } => assert_eq!(status, Some(400)),
            other => panic!("unexpected error: {}", other),
        }
    }

    #[test]
    fn stops_after_max_attempts() {
        let inner = SequenceClient::new(vec![
            Err(AppError::JulesApiError { message: "server error".to_string(), status: Some(500) }),
            Err(AppError::JulesApiError { message: "server error".to_string(), status: Some(500) }),
            Err(AppError::JulesApiError { message: "server error".to_string(), status: Some(500) }),
        ]);
        let client = RetryingJulesClient::new(Box::new(inner), policy(3));

        let result = client.create_session(test_request());
        assert!(result.is_err());
    }

    #[test]
    fn log_format_sanitizes_control_characters() {
        let err = AppError::JulesApiError {
            message: "bad\nerror\twith\rcontrols".to_string(),
            status: Some(500),
        };
        let formatted = format_error_for_log(&err);
        assert!(formatted.contains("JulesApiError(status=500):"));
        assert!(!formatted.contains('\n'));
        assert!(!formatted.contains('\r'));
    }
}
