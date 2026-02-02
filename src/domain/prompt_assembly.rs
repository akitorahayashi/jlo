//! Domain model for prompt assembly configuration.
//!
//! Prompt assembly is asset-driven: each layer has a `prompt_assembly.yml` that
//! declares runtime context variables and includes to be concatenated into the
//! final prompt.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Schema for `prompt_assembly.yml` files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptAssemblySpec {
    /// Schema version for forward compatibility.
    pub schema_version: u32,

    /// Layer this assembly belongs to.
    pub layer: String,

    /// Runtime context variables that must be provided at execution time.
    /// Keys are variable names (e.g., "workstream", "role"), values are
    /// placeholder patterns (e.g., "{{workstream}}").
    #[serde(default)]
    pub runtime_context: HashMap<String, String>,

    /// Ordered list of files to include in the assembled prompt.
    pub includes: Vec<PromptInclude>,
}

/// A single include directive in the prompt assembly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptInclude {
    /// Human-readable title for this section (used as section header).
    pub title: String,

    /// Path to the file to include, relative to workspace root.
    /// May contain `{{variable}}` placeholders to be substituted.
    pub path: String,

    /// Whether this include is optional (default: false).
    /// Missing optional includes are silently omitted.
    /// Missing required includes cause assembly to fail.
    #[serde(default)]
    pub optional: bool,
}

/// Runtime context for prompt assembly.
///
/// Contains the variable values to substitute into include paths.
#[derive(Debug, Clone, Default)]
pub struct PromptContext {
    /// Variable name to value mapping.
    pub variables: HashMap<String, String>,
}

impl PromptContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a variable to the context.
    pub fn with_var(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(name.into(), value.into());
        self
    }

    /// Get a variable value.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.variables.get(name).map(|s| s.as_str())
    }
}

/// Result of prompt assembly.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AssembledPrompt {
    /// The fully assembled prompt text.
    pub content: String,

    /// Files that were included (for diagnostic output).
    pub included_files: Vec<String>,

    /// Files that were skipped (optional and missing).
    pub skipped_files: Vec<String>,
}

/// Error during prompt assembly.
#[derive(Debug, Clone)]
pub enum PromptAssemblyError {
    /// The prompt_assembly.yml file was not found.
    AssemblySpecNotFound(String),

    /// Failed to parse the prompt_assembly.yml file.
    InvalidAssemblySpec { path: String, reason: String },

    /// A required runtime context variable was not provided.
    MissingContextVariable { variable: String, required_by: String },

    /// A required include file was not found.
    RequiredIncludeNotFound { path: String, title: String },

    /// Failed to read an include file.
    IncludeReadError { path: String, reason: String },

    /// The prompt.yml file was not found (for base prompt).
    PromptNotFound(String),

    /// Failed to read prompt.yml.
    PromptReadError { path: String, reason: String },

    /// Template syntax is not allowed in this context.
    TemplateSyntaxNotAllowed { template: String, token: String },

    /// Failed to render a template with the provided context.
    TemplateRenderError { template: String, reason: String },
}

impl std::fmt::Display for PromptAssemblyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AssemblySpecNotFound(path) => {
                write!(f, "Prompt assembly spec not found: {}", path)
            }
            Self::InvalidAssemblySpec { path, reason } => {
                write!(f, "Invalid prompt assembly spec at {}: {}", path, reason)
            }
            Self::MissingContextVariable { variable, required_by } => {
                write!(
                    f,
                    "Missing required context variable '{}' (required by {})",
                    variable, required_by
                )
            }
            Self::RequiredIncludeNotFound { path, title } => {
                write!(f, "Required include '{}' not found: {}", title, path)
            }
            Self::IncludeReadError { path, reason } => {
                write!(f, "Failed to read include {}: {}", path, reason)
            }
            Self::PromptNotFound(path) => {
                write!(f, "Prompt not found: {}", path)
            }
            Self::PromptReadError { path, reason } => {
                write!(f, "Failed to read prompt {}: {}", path, reason)
            }
            Self::TemplateSyntaxNotAllowed { template, token } => {
                write!(f, "Template syntax '{}' is not allowed in {}", token, template)
            }
            Self::TemplateRenderError { template, reason } => {
                write!(f, "Failed to render template {}: {}", template, reason)
            }
        }
    }
}

impl std::error::Error for PromptAssemblyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_context_with_var() {
        let ctx =
            PromptContext::new().with_var("workstream", "generic").with_var("role", "taxonomy");

        assert_eq!(ctx.get("workstream"), Some("generic"));
        assert_eq!(ctx.get("role"), Some("taxonomy"));
        assert_eq!(ctx.get("missing"), None);
    }

    #[test]
    fn prompt_assembly_spec_deserialize() {
        let yaml = r#"
schema_version: 1
layer: observers

runtime_context:
  workstream: "{{workstream}}"
  role: "{{role}}"

includes:
  - title: "Role"
    path: ".jules/roles/observers/roles/{{role}}/role.yml"
  - title: "Layer Contracts"
    path: ".jules/roles/observers/contracts.yml"
  - title: "Change Summary"
    path: ".jules/changes/latest.yml"
    optional: true
"#;

        let spec: PromptAssemblySpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(spec.schema_version, 1);
        assert_eq!(spec.layer, "observers");
        assert_eq!(spec.runtime_context.len(), 2);
        assert_eq!(spec.includes.len(), 3);
        assert!(!spec.includes[0].optional);
        assert!(!spec.includes[1].optional);
        assert!(spec.includes[2].optional);
    }
}
