//! Setup domain: component definitions, dependency resolution, and artifact generation.

pub mod artifact_generator;
pub mod dependency_graph;
pub mod error;
pub mod setup_component;
pub mod tools_config;

pub use artifact_generator::SetupEnvArtifacts;
pub use dependency_graph::DependencyGraph;
pub use error::SetupError;
pub use setup_component::{EnvSpec, SetupComponent, SetupComponentId};
