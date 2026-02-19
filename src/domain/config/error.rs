/// Configuration capability error.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}
