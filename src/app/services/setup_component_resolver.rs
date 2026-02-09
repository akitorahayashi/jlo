//! Setup component resolver service.

use crate::domain::component_graph::ComponentGraph;
use crate::domain::{AppError, Component};
use crate::ports::ComponentCatalog;

/// Service for resolving component dependencies for setup.
pub struct SetupComponentResolver;

impl SetupComponentResolver {
    /// Resolve dependencies and return components in installation order.
    ///
    /// Delegates to domain logic.
    pub fn resolve<C: ComponentCatalog>(
        requested: &[String],
        catalog: &C,
    ) -> Result<Vec<Component>, AppError> {
        ComponentGraph::resolve(requested, catalog)
    }
}
