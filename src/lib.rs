//! jlo: Deploy and manage .jules/ workspace scaffolding for organizational memory.

pub(crate) mod app;
pub(crate) mod domain;
pub(crate) mod ports;
pub(crate) mod services;

#[cfg(test)]
pub(crate) mod testing;

pub use app::api::*;

/// Entry point for the CLI.
pub use app::cli::run as cli;
