//! Dependency resolver service.

use crate::domain::dependency_graph::DependencyGraph;
use crate::domain::{AppError, Component};
use crate::ports::ComponentCatalog;

/// Service for resolving component dependencies using topological sort.
pub struct DependencyResolver;

impl DependencyResolver {
    /// Resolve dependencies and return components in installation order.
    ///
    /// Delegates to domain logic.
    pub fn resolve<C: ComponentCatalog>(
        requested: &[String],
        catalog: &C,
    ) -> Result<Vec<Component>, AppError> {
        DependencyGraph::resolve(requested, catalog)
    }
}
