//! Repository-backed configuration loaders and DTOs.
//!
//! This module normalizes app-level configuration access:
//! - repository/environment/system-backed loading
//! - conversion to domain models
//!
//! Pure schema/model parsing lives in `domain::configuration` and
//! `domain::setup`.

mod detect_repository_source;
mod load_config;
mod load_schedule;
mod load_setup_config;
mod mock;

pub use detect_repository_source::detect_repository_source;
pub use load_config::load_config;
pub use load_schedule::load_schedule;
pub use load_setup_config::load_setup_config;
pub use mock::{load_mock_config, validate_mock_prerequisites};
