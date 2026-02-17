//! jlo: Deploy and manage .jules/ runtime scaffolding for organizational memory.

pub mod adapters;
pub(crate) mod app;
pub mod domain;
pub mod ports;

#[cfg(test)]
pub(crate) mod testing;

pub use app::api::*;

/// Entry point for the CLI.
pub use app::cli::run as cli;
