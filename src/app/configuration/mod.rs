//! Repository-backed configuration readers.
//!
//! This module owns all I/O for loading configuration, schedules,
//! mock config, and repository source detection. Pure parsing
//! lives in `domain::configuration::*_parser` modules.

mod mock_config_reader;
mod repository_source_resolver;
mod run_config_reader;
mod schedule_reader;

pub use mock_config_reader::{load_mock_config, validate_mock_prerequisites};
pub use repository_source_resolver::detect_repository_source;
pub use run_config_reader::load_config;
pub use schedule_reader::{list_subdirectories, load_schedule};
