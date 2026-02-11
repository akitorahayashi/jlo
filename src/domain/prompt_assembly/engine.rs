//! Domain model for prompt assembly configuration.
//!
//! Prompt assembly is asset-driven: each layer has a `<layer>_prompt.j2` template
//! that renders the final prompt using safe include helpers.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use minijinja::{Environment, UndefinedBehavior};

use crate::domain::Layer;
use crate::domain::workspace::paths::jules;

/// Abstraction for prompt asset loading.
pub trait PromptAssetLoader {
    fn read_asset(&self, path: &Path) -> std::io::Result<String>;
    fn asset_exists(&self, path: &Path) -> bool;
    fn ensure_asset_dir(&self, path: &Path) -> std::io::Result<()>;
    fn copy_asset(&self, from: &Path, to: &Path) -> std::io::Result<u64>;
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
    #[allow(dead_code)]
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
    /// The prompt template file was not found.
    AssemblyTemplateNotFound(String),

    /// Failed to read the prompt template file.
    TemplateReadError { path: String, reason: String },

    /// A required include file was not found.
    RequiredIncludeNotFound { path: String, title: String },

    /// Failed to read an include file.
    IncludeReadError { path: String, reason: String },

    /// Failed to render a template with the provided context.
    TemplateRenderError { template: String, reason: String },

    /// Path traversal detected in include path.
    PathTraversalDetected { path: String },

    /// Failed to seed a missing file from a schema template.
    SchemaSeedError { path: String, reason: String },
}

impl std::fmt::Display for PromptAssemblyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AssemblyTemplateNotFound(path) => {
                write!(f, "Prompt assembly template not found: {}", path)
            }
            Self::TemplateReadError { path, reason } => {
                write!(f, "Failed to read prompt assembly template {}: {}", path, reason)
            }
            Self::RequiredIncludeNotFound { path, title } => {
                write!(f, "Required include '{}' not found: {}", title, path)
            }
            Self::IncludeReadError { path, reason } => {
                write!(f, "Failed to read include {}: {}", path, reason)
            }
            Self::TemplateRenderError { template, reason } => {
                write!(f, "Failed to render template {}: {}", template, reason)
            }
            Self::PathTraversalDetected { path } => {
                write!(f, "Path traversal detected in include path: {}", path)
            }
            Self::SchemaSeedError { path, reason } => {
                write!(f, "Failed to seed include {}: {}", path, reason)
            }
        }
    }
}

impl std::error::Error for PromptAssemblyError {}

/// Assemble a prompt for the given layer using the prompt assembly spec.
///
/// For multi-role layers (observers, innovators), the context must include
/// the `role` variable. For single-role layers, the context
/// may be empty.
///
/// For issue-driven layers (planners, implementers), use `assemble_with_issue`
/// to append issue content to the assembled prompt.
pub fn assemble_prompt<L>(
    jules_path: &Path,
    layer: Layer,
    context: &PromptContext,
    loader: &L,
) -> Result<AssembledPrompt, PromptAssemblyError>
where
    L: PromptAssetLoader + Clone + Send + Sync + 'static,
{
    let layer_dir = jules::layer_dir(jules_path, layer);
    let root = jules_path.parent().unwrap_or(Path::new("."));

    // Load prompt template
    let assembly_path = jules::prompt_template(jules_path, layer);
    if !loader.asset_exists(&assembly_path) {
        return Err(PromptAssemblyError::AssemblyTemplateNotFound(
            assembly_path.display().to_string(),
        ));
    }

    let template = loader.read_asset(&assembly_path).map_err(|err| {
        PromptAssemblyError::TemplateReadError {
            path: assembly_path.display().to_string(),
            reason: err.to_string(),
        }
    })?;

    let included_files = Arc::new(Mutex::new(Vec::new()));
    let skipped_files = Arc::new(Mutex::new(Vec::new()));
    let failure = Arc::new(Mutex::new(None));

    let include_ctx = Arc::new(IncludeContext {
        root: root.to_path_buf(),
        layer_dir: layer_dir.clone(),
        loader: Box::new(loader.clone()),
        included_files: included_files.clone(),
        skipped_files: skipped_files.clone(),
        failure: failure.clone(),
    });

    let mut env = Environment::new();
    env.set_keep_trailing_newline(true);
    env.set_undefined_behavior(UndefinedBehavior::Strict);

    {
        let include_ctx = include_ctx.clone();
        env.add_function("include_required", move |path: String| -> String {
            include_ctx.include_file(&path, true).unwrap_or_default()
        });
    }

    {
        let include_ctx = include_ctx.clone();
        env.add_function("include_optional", move |path: String| -> String {
            include_ctx.include_file(&path, false).unwrap_or_default()
        });
    }

    {
        let include_ctx = include_ctx.clone();
        env.add_function("file_exists", move |path: String| -> bool {
            if validate_safe_path(&path).is_err() {
                return false;
            }
            let full_path = include_ctx.root.join(&path);
            include_ctx.loader.asset_exists(&full_path)
        });
    }

    env.add_function("section", |title: String, content: String| -> String {
        if content.trim().is_empty() {
            return String::new();
        }
        format!("---\n# {}\n{}", title, content.trim_end())
    });

    let rendered = env.render_str(&template, &context.variables).map_err(|err| {
        PromptAssemblyError::TemplateRenderError {
            template: assembly_path.display().to_string(),
            reason: err.to_string(),
        }
    })?;

    if let Some(err) = failure.lock().unwrap().take() {
        return Err(err);
    }

    Ok(AssembledPrompt {
        content: rendered,
        included_files: included_files.lock().unwrap().clone(),
        skipped_files: skipped_files.lock().unwrap().clone(),
    })
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
pub fn assemble_with_issue<L>(
    jules_path: &Path,
    layer: Layer,
    issue_content: &str,
    loader: &L,
) -> Result<AssembledPrompt, PromptAssemblyError>
where
    L: PromptAssetLoader + Clone + Send + Sync + 'static,
{
    let mut result = assemble_prompt(jules_path, layer, &PromptContext::new(), loader)?;

    result.content.push_str(&format!("\n---\n# Issue\n{}", issue_content));
    result.included_files.push("(issue content embedded)".to_string());

    Ok(result)
}

struct IncludeContext {
    root: PathBuf,
    layer_dir: PathBuf,
    loader: Box<dyn PromptAssetLoader + Send + Sync>,
    included_files: Arc<Mutex<Vec<String>>>,
    skipped_files: Arc<Mutex<Vec<String>>>,
    failure: Arc<Mutex<Option<PromptAssemblyError>>>,
}

impl IncludeContext {
    fn include_file(&self, path: &str, required: bool) -> Option<String> {
        if self.failure.lock().unwrap().is_some() {
            return None;
        }

        if let Err(err) = validate_safe_path(path) {
            self.failure.lock().unwrap().replace(err);
            return None;
        }

        let full_path = self.root.join(path);
        self.seed_from_schema(path, &full_path, required);

        if self.failure.lock().unwrap().is_some() {
            return None;
        }

        if self.loader.asset_exists(&full_path) {
            match self.loader.read_asset(&full_path) {
                Ok(content) => {
                    self.included_files.lock().unwrap().push(path.to_string());
                    Some(content)
                }
                Err(err) => {
                    if required {
                        self.failure.lock().unwrap().replace(
                            PromptAssemblyError::IncludeReadError {
                                path: path.to_string(),
                                reason: err.to_string(),
                            },
                        );
                    } else {
                        self.skipped_files
                            .lock()
                            .unwrap()
                            .push(format!("{} (read error: {})", path, err));
                    }
                    None
                }
            }
        } else if required {
            self.failure.lock().unwrap().replace(PromptAssemblyError::RequiredIncludeNotFound {
                path: path.to_string(),
                title: path.to_string(),
            });
            None
        } else {
            self.skipped_files.lock().unwrap().push(format!("{} (not found)", path));
            None
        }
    }

    fn seed_from_schema(&self, path: &str, full_path: &Path, required: bool) {
        if self.loader.asset_exists(full_path) {
            return;
        }

        let Some(file_name) = Path::new(path).file_name() else {
            return;
        };

        let schema_path = self.layer_dir.join("schemas").join(file_name);
        if !self.loader.asset_exists(&schema_path) {
            return;
        }

        if let Some(parent) = full_path.parent()
            && let Err(err) = self.loader.ensure_asset_dir(parent)
        {
            if required {
                self.failure.lock().unwrap().replace(PromptAssemblyError::SchemaSeedError {
                    path: path.to_string(),
                    reason: err.to_string(),
                });
            }
            return;
        }

        if required {
            if let Err(err) = self.loader.copy_asset(&schema_path, full_path) {
                self.failure.lock().unwrap().replace(PromptAssemblyError::SchemaSeedError {
                    path: path.to_string(),
                    reason: err.to_string(),
                });
            }
        } else {
            let _ = self.loader.copy_asset(&schema_path, full_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
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
        let ctx = PromptContext::new().with_var("role", "taxonomy");

        assert_eq!(ctx.get("role"), Some("taxonomy"));
        assert_eq!(ctx.get("missing"), None);
    }

    #[test]
    fn test_assemble_prompt_mock_loader() {
        let mock_loader = MockPromptLoader::new();
        let jules_path = Path::new(".jules");

        // Setup mock files for Planners layer
        mock_loader.add_file(
                        ".jules/roles/planners/planner_prompt.j2",
                        r#"{{ section("Contracts", include_required(".jules/roles/planners/contracts.yml")) }}"#,
        );
        mock_loader
            .add_file(".jules/roles/planners/contracts.yml", "layer: planners\nconstraints: []");

        let result =
            assemble_prompt(jules_path, Layer::Planners, &PromptContext::new(), &mock_loader);

        assert!(result.is_ok());
        let assembled = result.unwrap();
        assert!(assembled.content.contains("# Contracts"));
        assert!(assembled.content.contains("layer: planners"));
    }

    #[test]
    fn test_assemble_prompt_optional_include_skipped() {
        let mock_loader = MockPromptLoader::new();
        let jules_path = Path::new(".jules");

        mock_loader.add_file(
            ".jules/roles/observers/observers_prompt.j2",
            r#"{{ section("Optional", include_optional(".jules/exchange/changes.yml")) }}"#,
        );

        let ctx = PromptContext::new().with_var("role", "qa");
        let result = assemble_prompt(jules_path, Layer::Observers, &ctx, &mock_loader).unwrap();

        assert!(!result.content.contains("# Optional"));
        assert!(result.skipped_files.iter().any(|entry| entry.contains("changes.yml")));
    }

    #[test]
    fn test_assemble_prompt_missing_required_include_fails() {
        let mock_loader = MockPromptLoader::new();
        let jules_path = Path::new(".jules");

        mock_loader.add_file(
            ".jules/roles/planners/planner_prompt.j2",
            r#"{{ section("Missing", include_required(".jules/roles/planners/contracts.yml")) }}"#,
        );

        let result =
            assemble_prompt(jules_path, Layer::Planners, &PromptContext::new(), &mock_loader);

        assert!(matches!(result, Err(PromptAssemblyError::RequiredIncludeNotFound { .. })));
    }

    #[test]
    fn test_assemble_prompt_schema_seed() {
        let mock_loader = MockPromptLoader::new();
        let jules_path = Path::new(".jules");

        mock_loader.add_file(
            ".jules/roles/observers/observers_prompt.j2",
            r#"{{ section("Perspective", include_required(".jules/workstations/taxonomy/perspective.yml")) }}"#,
        );
        mock_loader
            .add_file(".jules/roles/observers/schemas/perspective.yml", "schema: perspective");

        let ctx = PromptContext::new().with_var("role", "taxonomy");
        let result = assemble_prompt(jules_path, Layer::Observers, &ctx, &mock_loader).unwrap();

        assert!(result.content.contains("schema: perspective"));
        assert!(result.included_files.iter().any(|path| path.ends_with("perspective.yml")));
    }
}
