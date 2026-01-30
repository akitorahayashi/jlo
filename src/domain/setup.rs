//! Setup compiler domain models.

use serde::Deserialize;

/// Environment variable specification for a component.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EnvSpec {
    /// Variable name.
    pub name: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Default value (if any).
    #[serde(default)]
    pub default: Option<String>,
}

/// A component that can be installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// Component name (unique identifier).
    pub name: String,
    /// Short summary of what this component provides.
    pub summary: String,
    /// Names of components this depends on.
    pub dependencies: Vec<String>,
    /// Environment variables this component uses.
    pub env: Vec<EnvSpec>,
    /// Installation script content.
    pub script_content: String,
}

/// Configuration for setup script generation.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SetupConfig {
    /// List of tool names to install.
    #[serde(default)]
    pub tools: Vec<String>,
}

/// Metadata parsed from meta.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct ComponentMeta {
    /// Component name (defaults to directory name if missing).
    pub name: Option<String>,
    /// Short summary.
    #[serde(default)]
    pub summary: String,
    /// Dependencies list.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Environment specifications.
    #[serde(default)]
    pub env: Vec<EnvSpec>,
}

impl Component {
    /// Create a component from metadata and script content.
    pub fn from_meta(dir_name: &str, meta: ComponentMeta, script_content: String) -> Self {
        Self {
            name: meta.name.unwrap_or_else(|| dir_name.to_string()),
            summary: meta.summary,
            dependencies: meta.dependencies,
            env: meta.env,
            script_content,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_meta_uses_name_if_present() {
        let meta = ComponentMeta {
            name: Some("custom-name".to_string()),
            summary: "A test component".to_string(),
            dependencies: vec![],
            env: vec![],
        };
        let script = "echo hello".to_string();
        let component = Component::from_meta("dir-name", meta, script.clone());

        assert_eq!(component.name, "custom-name");
        assert_eq!(component.summary, "A test component");
        assert_eq!(component.script_content, script);
    }

    #[test]
    fn from_meta_uses_dirname_if_name_missing() {
        let meta = ComponentMeta {
            name: None,
            summary: "A test component".to_string(),
            dependencies: vec![],
            env: vec![],
        };
        let script = "echo hello".to_string();
        let component = Component::from_meta("dir-name", meta, script);

        assert_eq!(component.name, "dir-name");
    }

    #[test]
    fn from_meta_passes_through_fields() {
        let env_spec = EnvSpec {
            name: "TEST_VAR".to_string(),
            description: "A test variable".to_string(),
            default: Some("default".to_string()),
        };
        let meta = ComponentMeta {
            name: None,
            summary: "Summary".to_string(),
            dependencies: vec!["dep1".to_string(), "dep2".to_string()],
            env: vec![env_spec.clone()],
        };
        let component = Component::from_meta("test", meta, "".to_string());

        assert_eq!(component.dependencies, vec!["dep1", "dep2"]);
        assert_eq!(component.env, vec![env_spec]);
    }
}
