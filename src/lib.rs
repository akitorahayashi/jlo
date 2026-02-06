//! jlo: Deploy and manage .jules/ workspace scaffolding for organizational memory.

pub(crate) mod adapters;
pub(crate) mod api;
pub(crate) mod app;
pub(crate) mod domain;
pub(crate) mod ports;

#[cfg(test)]
pub(crate) mod testing;

pub use api::*;

/// Entry point for the CLI.
pub use app::cli::run as cli;
