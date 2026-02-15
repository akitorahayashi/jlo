//! Setup component catalog service - loads setup components from embedded assets.

use include_dir::{Dir, include_dir};
use serde::Deserialize;
use std::collections::BTreeMap;

use crate::domain::{AppError, EnvSpec, SetupComponent, SetupComponentId};
use crate::ports::SetupComponentCatalog;

/// Embedded setup component directory.
static CATALOG_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/assets/setup");

/// Metadata parsed from meta.toml.
#[derive(Debug, Deserialize)]
struct SetupComponentMeta {
    /// Component name (defaults to directory name if missing).
    pub name: Option<String>,
    /// Short summary.
    #[serde(default)]
    pub summary: String,
    /// Dependencies list.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Non-secret environment specifications.
    #[serde(default)]
    pub vars: BTreeMap<String, EnvValueSpec>,
    /// Secret environment specifications.
    #[serde(default)]
    pub secrets: BTreeMap<String, EnvValueSpec>,
}

/// Value spec used by `[vars]` and `[secrets]` metadata sections.
#[derive(Debug, Clone, Deserialize)]
struct EnvValueSpec {
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Default value (if any).
    #[serde(default)]
    pub default: Option<String>,
}

/// Service for managing the setup component catalog.
pub struct EmbeddedSetupComponentCatalog {
    components: BTreeMap<String, SetupComponent>,
}

impl EmbeddedSetupComponentCatalog {
    /// Create a new catalog by loading all embedded setup components.
    pub fn new() -> Result<Self, AppError> {
        let mut components = BTreeMap::new();

        for entry in CATALOG_DIR.dirs() {
            let dir_name = entry.path().file_name().and_then(|n| n.to_str()).unwrap_or("");

            let meta_file = entry.get_file(entry.path().join("meta.toml"));
            let script_file = entry.get_file(entry.path().join("install.sh"));

            let (Some(meta_file), Some(script_file)) = (meta_file, script_file) else {
                continue;
            };

            let meta_content = meta_file.contents_utf8().ok_or_else(|| {
                AppError::InvalidSetupComponentMetadata {
                    component: dir_name.to_string(),
                    reason: "meta.toml is not valid UTF-8".to_string(),
                }
            })?;

            let script_content = script_file.contents_utf8().ok_or_else(|| {
                AppError::InvalidSetupComponentMetadata {
                    component: dir_name.to_string(),
                    reason: "install.sh is not valid UTF-8".to_string(),
                }
            })?;

            let meta: SetupComponentMeta = toml::from_str(meta_content).map_err(|e| {
                AppError::InvalidSetupComponentMetadata {
                    component: dir_name.to_string(),
                    reason: e.to_string(),
                }
            })?;

            let name_str = meta.name.clone().unwrap_or_else(|| dir_name.to_string());
            let name_id = SetupComponentId::new(&name_str).map_err(|_| {
                AppError::InvalidSetupComponentMetadata {
                    component: dir_name.to_string(),
                    reason: format!("Invalid setup component name '{}'", name_str),
                }
            })?;

            let mut dependencies = Vec::new();
            for dep in &meta.dependencies {
                dependencies.push(SetupComponentId::new(dep).map_err(|_| {
                    AppError::InvalidSetupComponentMetadata {
                        component: dir_name.to_string(),
                        reason: format!("Invalid dependency name '{}'", dep),
                    }
                })?);
            }

            if let Some(duplicate_key) =
                meta.vars.keys().find(|key| meta.secrets.contains_key(*key))
            {
                return Err(AppError::InvalidSetupComponentMetadata {
                    component: dir_name.to_string(),
                    reason: format!(
                        "Environment key '{}' is declared in both [vars] and [secrets]",
                        duplicate_key
                    ),
                });
            }

            let mut env = Vec::new();
            for (name, spec) in &meta.vars {
                env.push(EnvSpec {
                    name: name.clone(),
                    description: spec.description.clone(),
                    default: spec.default.clone(),
                    secret: false,
                });
            }
            for (name, spec) in &meta.secrets {
                env.push(EnvSpec {
                    name: name.clone(),
                    description: spec.description.clone(),
                    default: spec.default.clone(),
                    secret: true,
                });
            }

            let component = SetupComponent {
                name: name_id,
                summary: meta.summary,
                dependencies,
                env,
                script_content: script_content.to_string(),
            };

            components.insert(component.name.to_string(), component);
        }

        Ok(Self { components })
    }
}

impl Default for EmbeddedSetupComponentCatalog {
    fn default() -> Self {
        Self::new().expect("Failed to load embedded setup component catalog")
    }
}

impl SetupComponentCatalog for EmbeddedSetupComponentCatalog {
    fn get(&self, name: &str) -> Option<&SetupComponent> {
        self.components.get(name)
    }

    fn list_all(&self) -> Vec<&SetupComponent> {
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
        let catalog = EmbeddedSetupComponentCatalog::new().unwrap();
        let names = catalog.names();

        assert!(names.contains(&"gh"), "should contain 'gh' component");
        assert!(names.contains(&"just"), "should contain 'just' component");
        assert!(names.contains(&"swift"), "should contain 'swift' component");
        assert!(names.contains(&"uv"), "should contain 'uv' component");
    }

    #[test]
    fn get_component_by_name() {
        let catalog = EmbeddedSetupComponentCatalog::new().unwrap();
        let just = catalog.get("just").expect("just should exist");

        assert_eq!(just.name.as_str(), "just");
        assert!(!just.summary.is_empty());
        assert!(!just.script_content.is_empty());
    }

    #[test]
    fn list_all_returns_sorted() {
        let catalog = EmbeddedSetupComponentCatalog::new().unwrap();
        let all = catalog.list_all();

        assert!(all.len() >= 4);
        // BTreeMap maintains order.
        let names: Vec<_> = all.iter().map(|c| c.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }
}
