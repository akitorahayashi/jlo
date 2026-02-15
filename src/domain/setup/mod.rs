//! Setup domain: component definitions, dependency resolution, and artifact generation.

pub mod artifact_generator;
pub mod dependency_graph;
pub mod setup_component;

pub use artifact_generator::SetupEnvArtifacts;
pub use dependency_graph::DependencyGraph;
pub use setup_component::{EnvSpec, SetupComponent, SetupComponentId};
