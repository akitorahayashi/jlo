//! Setup list command - lists available components.

use crate::domain::AppError;
use crate::ports::ComponentCatalog;

/// Summary information for a component.
#[derive(Debug, Clone)]
pub struct ComponentSummary {
    pub name: String,
    pub summary: String,
}

/// Detailed information for a component.
#[derive(Debug, Clone)]
pub struct ComponentDetail {
    pub name: String,
    pub summary: String,
    pub dependencies: Vec<String>,
    pub env_vars: Vec<EnvVarInfo>,
    pub script_content: String,
}

/// Environment variable information.
#[derive(Debug, Clone)]
pub struct EnvVarInfo {
    pub name: String,
    pub description: String,
    pub default: Option<String>,
}

/// Execute the setup list command.
///
/// Returns summaries of all available components.
pub fn execute(catalog: &impl ComponentCatalog) -> Result<Vec<ComponentSummary>, AppError> {
    let components = catalog.list_all();

    Ok(components
        .into_iter()
        .map(|c| ComponentSummary { name: c.name.to_string(), summary: c.summary.clone() })
        .collect())
}

/// Execute the setup list --detail command.
///
/// Returns detailed information for a specific component.
pub fn execute_detail(
    catalog: &impl ComponentCatalog,
    component_name: &str,
) -> Result<ComponentDetail, AppError> {
    let component = catalog.get(component_name).ok_or_else(|| AppError::ComponentNotFound {
        name: component_name.to_string(),
        available: catalog.names().iter().map(|s| s.to_string()).collect(),
    })?;

    Ok(ComponentDetail {
        name: component.name.to_string(),
        summary: component.summary.clone(),
        dependencies: component.dependencies.iter().map(|d| d.to_string()).collect(),
        env_vars: component
            .env
            .iter()
            .map(|e| EnvVarInfo {
                name: e.name.clone(),
                description: e.description.clone(),
                default: e.default.clone(),
            })
            .collect(),
        script_content: component.script_content.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::assets::component_catalog_embedded::EmbeddedComponentCatalog;

    fn test_catalog() -> EmbeddedComponentCatalog {
        EmbeddedComponentCatalog::new().unwrap()
    }

    #[test]
    fn list_returns_components() {
        let catalog = test_catalog();
        let result = execute(&catalog).unwrap();

        assert!(!result.is_empty());
        assert!(result.iter().any(|c| c.name == "just"));
        assert!(result.iter().any(|c| c.name == "swift"));
        assert!(result.iter().any(|c| c.name == "uv"));
    }

    #[test]
    fn detail_returns_component_info() {
        let catalog = test_catalog();
        let result = execute_detail(&catalog, "just").unwrap();

        assert_eq!(result.name, "just");
        assert!(!result.summary.is_empty());
        assert!(!result.script_content.is_empty());
    }

    #[test]
    fn detail_not_found() {
        let catalog = test_catalog();
        let result = execute_detail(&catalog, "nonexistent");

        assert!(matches!(result, Err(AppError::ComponentNotFound { .. })));
    }
}
