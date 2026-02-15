#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    #[error("Schedule config invalid: {0}")]
    ConfigInvalid(String),

    #[error("TOML format error: {0}")]
    Toml(#[from] toml::de::Error),
}
