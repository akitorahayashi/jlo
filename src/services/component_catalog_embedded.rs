//! Component catalog service - loads components from embedded assets.

use include_dir::{Dir, include_dir};

use std::collections::BTreeMap;

use crate::app::config::ComponentMeta;
use crate::domain::{AppError, Component, ComponentId};
use crate::ports::ComponentCatalog;

/// Embedded catalog directory.
static CATALOG_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/assets/catalog");

/// Service for managing the component catalog.
pub struct EmbeddedComponentCatalog {
    components: BTreeMap<String, Component>,
}

impl EmbeddedComponentCatalog {
    /// Create a new catalog by loading all embedded components.
    pub fn new() -> Result<Self, AppError> {
        let mut components = BTreeMap::new();

        for entry in CATALOG_DIR.dirs() {
            let dir_name = entry.path().file_name().and_then(|n| n.to_str()).unwrap_or("");

            let meta_file = entry.get_file(entry.path().join("meta.toml"));
            let script_file = entry.get_file(entry.path().join("install.sh"));

            let (Some(meta_file), Some(script_file)) = (meta_file, script_file) else {
                continue;
            };

            let meta_content =
                meta_file.contents_utf8().ok_or_else(|| AppError::InvalidComponentMetadata {
                    component: dir_name.to_string(),
                    reason: "meta.toml is not valid UTF-8".to_string(),
                })?;

            let script_content =
                script_file.contents_utf8().ok_or_else(|| AppError::InvalidComponentMetadata {
                    component: dir_name.to_string(),
                    reason: "install.sh is not valid UTF-8".to_string(),
                })?;

            let meta: ComponentMeta =
                toml::from_str(meta_content).map_err(|e| AppError::InvalidComponentMetadata {
                    component: dir_name.to_string(),
                    reason: e.to_string(),
                })?;

            let name_str = meta.name.clone().unwrap_or_else(|| dir_name.to_string());
            let name_id =
                ComponentId::new(&name_str).map_err(|_| AppError::InvalidComponentMetadata {
                    component: dir_name.to_string(),
                    reason: format!("Invalid component name '{}'", name_str),
                })?;

            let mut dependencies = Vec::new();
            for dep in &meta.dependencies {
                dependencies.push(ComponentId::new(dep).map_err(|_| {
                    AppError::InvalidComponentMetadata {
                        component: dir_name.to_string(),
                        reason: format!("Invalid dependency name '{}'", dep),
                    }
                })?);
            }

            let component = Component {
                name: name_id,
                summary: meta.summary,
                dependencies,
                env: meta.env,
                script_content: script_content.to_string(),
            };

            components.insert(component.name.to_string(), component);
        }

        Ok(Self { components })
    }
}

impl Default for EmbeddedComponentCatalog {
    fn default() -> Self {
        Self::new().expect("Failed to load embedded catalog")
    }
}

impl ComponentCatalog for EmbeddedComponentCatalog {
    fn get(&self, name: &str) -> Option<&Component> {
        self.components.get(name)
    }

    fn list_all(&self) -> Vec<&Component> {
        self.components.values().collect()
    }

    fn names(&self) -> Vec<&str> {
        self.components.keys().map(String::as_str).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_embedded_components() {
        let catalog = EmbeddedComponentCatalog::new().unwrap();
        let names = catalog.names();

        assert!(names.contains(&"just"), "should contain 'just' component");
        assert!(names.contains(&"swift"), "should contain 'swift' component");
        assert!(names.contains(&"uv"), "should contain 'uv' component");
    }

    #[test]
    fn get_component_by_name() {
        let catalog = EmbeddedComponentCatalog::new().unwrap();
        let just = catalog.get("just").expect("just should exist");

        assert_eq!(just.name.as_str(), "just");
        assert!(!just.summary.is_empty());
        assert!(!just.script_content.is_empty());
    }

    #[test]
    fn list_all_returns_sorted() {
        let catalog = EmbeddedComponentCatalog::new().unwrap();
        let all = catalog.list_all();

        assert!(all.len() >= 3);
        // BTreeMap maintains order
        let names: Vec<_> = all.iter().map(|c| c.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }
}
