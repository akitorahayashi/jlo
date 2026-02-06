//! Domain model for prompt assembly configuration.
//!
//! Prompt assembly is asset-driven: each layer has a `prompt_assembly.yml` that
//! declares runtime context variables and includes to be concatenated into the
//! final prompt.

use std::collections::HashMap;
use std::path::{Component, Path};
use std::sync::OnceLock;

use minijinja::{Environment, UndefinedBehavior};
use serde::{Deserialize, Serialize};

use crate::domain::Layer;

/// Abstraction for prompt asset loading.
pub trait PromptAssetLoader {
    fn read_asset(&self, path: &Path) -> std::io::Result<String>;
    fn asset_exists(&self, path: &Path) -> bool;
    fn ensure_asset_dir(&self, path: &Path) -> std::io::Result<()>;
    fn copy_asset(&self, from: &Path, to: &Path) -> std::io::Result<u64>;
}

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

    /// Path traversal detected in include path.
    PathTraversalDetected { path: String },
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
            Self::PathTraversalDetected { path } => {
                write!(f, "Path traversal detected in include path: {}", path)
            }
        }
    }
}

impl std::error::Error for PromptAssemblyError {}

/// Assemble a prompt for the given layer using the prompt assembly spec.
///
/// For multi-role layers (observers, deciders), the context must include
/// `workstream` and `role` variables. For single-role layers, the context
/// may be empty.
///
/// For issue-driven layers (planners, implementers), use `assemble_with_issue`
/// to append issue content to the assembled prompt.
pub fn assemble_prompt(
    jules_path: &Path,
    layer: Layer,
    context: &PromptContext,
    loader: &impl PromptAssetLoader,
) -> Result<AssembledPrompt, PromptAssemblyError> {
    let layer_dir = jules_path.join("roles").join(layer.dir_name());
    let root = jules_path.parent().unwrap_or(Path::new("."));

    // Load prompt_assembly.yml
    let assembly_path = layer_dir.join("prompt_assembly.yml");
    let spec = load_assembly_spec(&assembly_path, loader)?;

    // Validate required context variables
    validate_context(&spec, context)?;

    // Load base prompt.yml
    let prompt_path = layer_dir.join("prompt.yml");
    let base_prompt = load_prompt(&prompt_path, context, loader)?;

    // Assemble includes
    let mut parts = vec![base_prompt];
    let mut included_files = vec![prompt_path.display().to_string()];
    let mut skipped_files = Vec::new();

    for include in &spec.includes {
        let resolved_path = render_template(
            &include.path,
            context,
            &format!("prompt_assembly include path ({})", include.title),
        )?;
        validate_safe_path(&resolved_path)?;
        let full_path = root.join(&resolved_path);

        // Auto-initialize from schema if missing
        if !loader.asset_exists(&full_path)
            && let Some(file_name) = Path::new(&resolved_path).file_name()
        {
            let schema_path = layer_dir.join("schemas").join(file_name);
            if loader.asset_exists(&schema_path) {
                if let Some(parent) = full_path.parent() {
                    let _ = loader.ensure_asset_dir(parent);
                }
                let _ = loader.copy_asset(&schema_path, &full_path);
            }
        }

        if loader.asset_exists(&full_path) {
            match loader.read_asset(&full_path) {
                Ok(content) => {
                    parts.push(format!("\n---\n# {}\n{}", include.title, content));
                    included_files.push(resolved_path);
                }
                Err(err) => {
                    if include.optional {
                        skipped_files.push(format!("{} (read error: {})", resolved_path, err));
                    } else {
                        return Err(PromptAssemblyError::IncludeReadError {
                            path: resolved_path,
                            reason: err.to_string(),
                        });
                    }
                }
            }
        } else if include.optional {
            skipped_files.push(format!("{} (not found)", resolved_path));
        } else {
            return Err(PromptAssemblyError::RequiredIncludeNotFound {
                path: resolved_path,
                title: include.title.clone(),
            });
        }
    }

    Ok(AssembledPrompt { content: parts.join("\n"), included_files, skipped_files })
}

/// Validate that a path is safe and does not traverse outside the root.
fn validate_safe_path(path: &str) -> Result<(), PromptAssemblyError> {
    let p = Path::new(path);
    if p.is_absolute() {
        return Err(PromptAssemblyError::PathTraversalDetected { path: path.to_string() });
    }

    for component in p.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {}
            _ => {
                return Err(PromptAssemblyError::PathTraversalDetected { path: path.to_string() });
            }
        }
    }
    Ok(())
}

/// Assemble a prompt for an issue-driven layer (planners, implementers).
///
/// This appends the issue content to the base assembled prompt.
#[allow(dead_code)]
pub fn assemble_with_issue(
    jules_path: &Path,
    layer: Layer,
    issue_content: &str,
    loader: &impl PromptAssetLoader,
) -> Result<AssembledPrompt, PromptAssemblyError> {
    let mut result = assemble_prompt(jules_path, layer, &PromptContext::new(), loader)?;

    result.content.push_str(&format!("\n---\n# Issue\n{}", issue_content));
    result.included_files.push("(issue content embedded)".to_string());

    Ok(result)
}

/// Load and parse the prompt assembly spec from a file.
fn load_assembly_spec(
    path: &Path,
    loader: &impl PromptAssetLoader,
) -> Result<PromptAssemblySpec, PromptAssemblyError> {
    if !loader.asset_exists(path) {
        return Err(PromptAssemblyError::AssemblySpecNotFound(path.display().to_string()));
    }

    let content =
        loader.read_asset(path).map_err(|err| PromptAssemblyError::InvalidAssemblySpec {
            path: path.display().to_string(),
            reason: err.to_string(),
        })?;

    serde_yaml::from_str(&content).map_err(|err| PromptAssemblyError::InvalidAssemblySpec {
        path: path.display().to_string(),
        reason: err.to_string(),
    })
}

/// Validate that all required context variables are present.
fn validate_context(
    spec: &PromptAssemblySpec,
    context: &PromptContext,
) -> Result<(), PromptAssemblyError> {
    for var_name in spec.runtime_context.keys() {
        if context.get(var_name).is_none() {
            return Err(PromptAssemblyError::MissingContextVariable {
                variable: var_name.clone(),
                required_by: format!("prompt_assembly.yml (layer: {})", spec.layer),
            });
        }
    }
    Ok(())
}

/// Load the base prompt.yml and render templates.
fn load_prompt(
    path: &Path,
    context: &PromptContext,
    loader: &impl PromptAssetLoader,
) -> Result<String, PromptAssemblyError> {
    if !loader.asset_exists(path) {
        return Err(PromptAssemblyError::PromptNotFound(path.display().to_string()));
    }

    let content = loader.read_asset(path).map_err(|err| PromptAssemblyError::PromptReadError {
        path: path.display().to_string(),
        reason: err.to_string(),
    })?;

    render_template(&content, context, &path.display().to_string())
}

static ENV: OnceLock<Environment<'static>> = OnceLock::new();

/// Render a template string using strict Jinja-compatible semantics.
///
/// Only `{{ ... }}` interpolation is allowed. Control structures are rejected.
fn render_template(
    template: &str,
    context: &PromptContext,
    template_name: &str,
) -> Result<String, PromptAssemblyError> {
    if let Some(token) = disallowed_template_token(template) {
        return Err(PromptAssemblyError::TemplateSyntaxNotAllowed {
            template: template_name.to_string(),
            token: token.to_string(),
        });
    }

    let env = ENV.get_or_init(|| {
        let mut env = Environment::new();
        env.set_undefined_behavior(UndefinedBehavior::Strict);
        env
    });

    env.render_str(template, &context.variables)
        .map_err(|err| template_render_error(template_name, err))
}

fn disallowed_template_token(template: &str) -> Option<&'static str> {
    if template.contains("{%") {
        return Some("{%");
    }
    if template.contains("{#") {
        return Some("{#");
    }
    None
}

fn template_render_error(template_name: &str, err: impl std::fmt::Display) -> PromptAssemblyError {
    PromptAssemblyError::TemplateRenderError {
        template: template_name.to_string(),
        reason: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct MockPromptLoader {
        files: Arc<Mutex<HashMap<String, String>>>,
    }

    impl MockPromptLoader {
        fn new() -> Self {
            Self { files: Arc::new(Mutex::new(HashMap::new())) }
        }

        fn add_file(&self, path: &str, content: &str) {
            self.files.lock().unwrap().insert(path.to_string(), content.to_string());
        }
    }

    impl PromptAssetLoader for MockPromptLoader {
        fn read_asset(&self, path: &Path) -> std::io::Result<String> {
            let path_str = path.to_string_lossy().to_string();
            self.files
                .lock()
                .unwrap()
                .get(&path_str)
                .cloned()
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"))
        }

        fn asset_exists(&self, path: &Path) -> bool {
            let path_str = path.to_string_lossy().to_string();
            self.files.lock().unwrap().contains_key(&path_str)
        }

        fn ensure_asset_dir(&self, _path: &Path) -> std::io::Result<()> {
            Ok(())
        }

        fn copy_asset(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
            let from_str = from.to_string_lossy().to_string();
            let to_str = to.to_string_lossy().to_string();
            let mut files = self.files.lock().unwrap();
            let content = files.get(&from_str).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "Source file not found")
            })?;
            let content_clone = content.clone();
            let len = content_clone.len() as u64;
            files.insert(to_str, content_clone);
            Ok(len)
        }
    }

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

    #[test]
    fn test_assemble_prompt_mock_loader() {
        let mock_loader = MockPromptLoader::new();
        let jules_path = Path::new(".jules");

        // Setup mock files for Planners layer
        mock_loader.add_file(
            ".jules/roles/planners/prompt_assembly.yml",
            r#"
schema_version: 1
layer: planners
runtime_context: {}
includes:
  - title: "Contracts"
    path: ".jules/roles/planners/contracts.yml"
"#,
        );
        mock_loader.add_file(".jules/roles/planners/prompt.yml", "role: planners\nlayer: planners");
        mock_loader
            .add_file(".jules/roles/planners/contracts.yml", "layer: planners\nconstraints: []");

        let result =
            assemble_prompt(jules_path, Layer::Planners, &PromptContext::new(), &mock_loader);

        assert!(result.is_ok());
        let assembled = result.unwrap();
        assert!(assembled.content.contains("role: planners"));
        assert!(assembled.content.contains("# Contracts"));
        assert!(assembled.content.contains("layer: planners"));
    }
}
