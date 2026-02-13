//! Setup list command - lists available components.

use crate::adapters::assets::component_catalog_embedded::EmbeddedComponentCatalog;
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
pub fn execute() -> Result<Vec<ComponentSummary>, AppError> {
    let catalog = EmbeddedComponentCatalog::new()?;
    let components = catalog.list_all();

    Ok(components
        .into_iter()
        .map(|c| ComponentSummary { name: c.name.to_string(), summary: c.summary.clone() })
        .collect())
}

/// Execute the setup list --detail command.
///
/// Returns detailed information for a specific component.
pub fn execute_detail(component_name: &str) -> Result<ComponentDetail, AppError> {
    let catalog = EmbeddedComponentCatalog::new()?;

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

    #[test]
    fn list_returns_components() {
        let result = execute().unwrap();

        assert!(!result.is_empty());
        assert!(result.iter().any(|c| c.name == "gh"));
        assert!(result.iter().any(|c| c.name == "just"));
        assert!(result.iter().any(|c| c.name == "swift"));
        assert!(result.iter().any(|c| c.name == "uv"));
    }

    #[test]
    fn detail_returns_component_info() {
        let result = execute_detail("just").unwrap();

        assert_eq!(result.name, "just");
        assert!(!result.summary.is_empty());
        assert!(!result.script_content.is_empty());
    }

    #[test]
    fn detail_not_found() {
        let result = execute_detail("nonexistent");

        assert!(matches!(result, Err(AppError::ComponentNotFound { .. })));
    }
}
