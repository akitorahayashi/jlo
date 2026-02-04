//! Prompt assembly service.
//!
//! Loads `prompt_assembly.yml` from the workspace and assembles the final prompt
//! by reading the base `prompt.yml`, substituting placeholders, and concatenating
//! included files with section headers.

use std::fs;
use std::path::{Component, Path};
use std::sync::OnceLock;

use minijinja::{Environment, UndefinedBehavior};

use crate::domain::{
    AssembledPrompt, Layer, PromptAssemblyError, PromptAssemblySpec, PromptContext,
};

/// Abstraction for filesystem operations to enable testing.
pub trait PromptFs {
    fn read_to_string(&self, path: &Path) -> std::io::Result<String>;
    fn exists(&self, path: &Path) -> bool;
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()>;
    fn copy(&self, from: &Path, to: &Path) -> std::io::Result<u64>;
}

/// Real filesystem implementation using std::fs.
pub struct RealPromptFs;

impl PromptFs for RealPromptFs {
    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        fs::read_to_string(path)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        fs::create_dir_all(path)
    }

    fn copy(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        fs::copy(from, to)
    }
}

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
    fs_impl: &impl PromptFs,
) -> Result<AssembledPrompt, PromptAssemblyError> {
    let layer_dir = jules_path.join("roles").join(layer.dir_name());
    let root = jules_path.parent().unwrap_or(Path::new("."));

    // Load prompt_assembly.yml
    let assembly_path = layer_dir.join("prompt_assembly.yml");
    let spec = load_assembly_spec(&assembly_path, fs_impl)?;

    // Validate required context variables
    validate_context(&spec, context)?;

    // Load base prompt.yml
    let prompt_path = layer_dir.join("prompt.yml");
    let base_prompt = load_prompt(&prompt_path, context, fs_impl)?;

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
        if !fs_impl.exists(&full_path)
            && let Some(file_name) = Path::new(&resolved_path).file_name()
        {
            let schema_path = layer_dir.join("schemas").join(file_name);
            if fs_impl.exists(&schema_path) {
                if let Some(parent) = full_path.parent() {
                    let _ = fs_impl.create_dir_all(parent);
                }
                let _ = fs_impl.copy(&schema_path, &full_path);
            }
        }

        if fs_impl.exists(&full_path) {
            match fs_impl.read_to_string(&full_path) {
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
    fs_impl: &impl PromptFs,
) -> Result<AssembledPrompt, PromptAssemblyError> {
    let mut result = assemble_prompt(jules_path, layer, &PromptContext::new(), fs_impl)?;

    result.content.push_str(&format!("\n---\n# Issue\n{}", issue_content));
    result.included_files.push("(issue content embedded)".to_string());

    Ok(result)
}

/// Load and parse the prompt assembly spec from a file.
fn load_assembly_spec(
    path: &Path,
    fs_impl: &impl PromptFs,
) -> Result<PromptAssemblySpec, PromptAssemblyError> {
    if !fs_impl.exists(path) {
        return Err(PromptAssemblyError::AssemblySpecNotFound(path.display().to_string()));
    }

    let content =
        fs_impl.read_to_string(path).map_err(|err| PromptAssemblyError::InvalidAssemblySpec {
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
    fs_impl: &impl PromptFs,
) -> Result<String, PromptAssemblyError> {
    if !fs_impl.exists(path) {
        return Err(PromptAssemblyError::PromptNotFound(path.display().to_string()));
    }

    let content =
        fs_impl.read_to_string(path).map_err(|err| PromptAssemblyError::PromptReadError {
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
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    struct MockPromptFs {
        files: Arc<Mutex<HashMap<String, String>>>,
    }

    impl MockPromptFs {
        fn new() -> Self {
            Self { files: Arc::new(Mutex::new(HashMap::new())) }
        }

        fn add_file(&self, path: &str, content: &str) {
            self.files.lock().unwrap().insert(path.to_string(), content.to_string());
        }
    }

    impl PromptFs for MockPromptFs {
        fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
            let path_str = path.to_string_lossy().to_string();
            self.files
                .lock()
                .unwrap()
                .get(&path_str)
                .cloned()
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"))
        }

        fn exists(&self, path: &Path) -> bool {
            let path_str = path.to_string_lossy().to_string();
            self.files.lock().unwrap().contains_key(&path_str)
        }

        fn create_dir_all(&self, _path: &Path) -> std::io::Result<()> {
            Ok(())
        }

        fn copy(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
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

    // --- Original tests using RealPromptFs via std::fs ---

    fn setup_test_workspace(dir: &Path, layer: &str, single_role: bool) {
        let jules = dir.join(".jules");
        let layer_dir = jules.join("roles").join(layer);
        fs::create_dir_all(&layer_dir).unwrap();

        // Create prompt_assembly.yml
        let assembly = if single_role {
            format!(
                r#"schema_version: 1
layer: {}

runtime_context: {{}}

includes:
  - title: "Layer Contracts"
    path: ".jules/roles/{}/contracts.yml"
"#,
                layer, layer
            )
        } else {
            format!(
                r#"schema_version: 1
layer: {}

runtime_context:
  workstream: "{{{{workstream}}}}"
  role: "{{{{role}}}}"

includes:
  - title: "Role"
    path: ".jules/roles/{}/roles/{{{{role}}}}/role.yml"
  - title: "Layer Contracts"
    path: ".jules/roles/{}/contracts.yml"
  - title: "Optional File"
    path: ".jules/optional.yml"
    optional: true
"#,
                layer, layer, layer
            )
        };
        fs::write(layer_dir.join("prompt_assembly.yml"), assembly).unwrap();

        // Create prompt.yml
        let prompt = if single_role {
            format!(
                r#"role: {}
layer: {}

contracts:
  - .jules/JULES.md
  - .jules/roles/{}/contracts.yml
"#,
                layer, layer, layer
            )
        } else {
            format!(
                r#"role: observer
layer: {}

contracts:
  - .jules/JULES.md
  - .jules/roles/{}/contracts.yml
  - .jules/roles/{}/roles/{{{{role}}}}/role.yml
"#,
                layer, layer, layer
            )
        };
        fs::write(layer_dir.join("prompt.yml"), prompt).unwrap();

        // Create contracts.yml
        fs::write(layer_dir.join("contracts.yml"), "layer: test\nconstraints: []").unwrap();

        // For multi-role, create a role
        if !single_role {
            let role_dir = layer_dir.join("roles").join("test_role");
            fs::create_dir_all(&role_dir).unwrap();
            fs::write(role_dir.join("role.yml"), "role: test_role\nfocus: testing").unwrap();
        }
    }

    #[test]
    fn assemble_single_role_prompt() {
        let dir = tempdir().unwrap();
        setup_test_workspace(dir.path(), "planners", true);

        let jules_path = dir.path().join(".jules");
        let fs_impl = RealPromptFs;
        let result = assemble_prompt(&jules_path, Layer::Planners, &PromptContext::new(), &fs_impl);

        assert!(result.is_ok());
        let assembled = result.unwrap();
        assert!(assembled.content.contains("role: planners"));
        assert!(assembled.content.contains("# Layer Contracts"));
        assert!(assembled.included_files.len() >= 2);
    }

    #[test]
    fn assemble_multi_role_prompt() {
        let dir = tempdir().unwrap();
        setup_test_workspace(dir.path(), "observers", false);

        let jules_path = dir.path().join(".jules");
        let context =
            PromptContext::new().with_var("workstream", "generic").with_var("role", "test_role");
        let fs_impl = RealPromptFs;

        let result = assemble_prompt(&jules_path, Layer::Observers, &context, &fs_impl);

        assert!(result.is_ok());
        let assembled = result.unwrap();
        assert!(assembled.content.contains("role: observer"));
        assert!(assembled.content.contains("# Role"));
        assert!(assembled.content.contains("role: test_role"));
        // Optional file should be skipped
        assert!(!assembled.skipped_files.is_empty());
    }

    #[test]
    fn missing_context_variable_fails() {
        let dir = tempdir().unwrap();
        setup_test_workspace(dir.path(), "observers", false);

        let jules_path = dir.path().join(".jules");
        // Missing 'role' variable
        let context = PromptContext::new().with_var("workstream", "generic");
        let fs_impl = RealPromptFs;

        let result = assemble_prompt(&jules_path, Layer::Observers, &context, &fs_impl);

        assert!(result.is_err());
        match result.unwrap_err() {
            PromptAssemblyError::MissingContextVariable { variable, .. } => {
                assert_eq!(variable, "role");
            }
            other => panic!("Expected MissingContextVariable, got {:?}", other),
        }
    }

    // --- New test using MockPromptFs ---

    #[test]
    fn test_assemble_prompt_mock_fs() {
        let mock_fs = MockPromptFs::new();
        let jules_path = Path::new(".jules");

        // Setup mock files for Planners layer
        mock_fs.add_file(
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
        mock_fs.add_file(".jules/roles/planners/prompt.yml", "role: planners\nlayer: planners");
        mock_fs.add_file(".jules/roles/planners/contracts.yml", "layer: planners\nconstraints: []");

        let result = assemble_prompt(jules_path, Layer::Planners, &PromptContext::new(), &mock_fs);

        assert!(result.is_ok());
        let assembled = result.unwrap();
        assert!(assembled.content.contains("role: planners"));
        assert!(assembled.content.contains("# Contracts"));
        assert!(assembled.content.contains("layer: planners"));
    }

    #[test]
    fn render_template_replaces_all() {
        let template =
            "workstream: {{ workstream }}, role: {{role}}, path: {{workstream}}/{{role}}";
        let context =
            PromptContext::new().with_var("workstream", "generic").with_var("role", "taxonomy");

        let result = render_template(template, &context, "inline").unwrap();

        assert_eq!(result, "workstream: generic, role: taxonomy, path: generic/taxonomy");
    }

    #[test]
    fn render_template_rejects_control_syntax() {
        let template = "{% if true %}nope{% endif %}";
        let context = PromptContext::new();

        let result = render_template(template, &context, "inline").unwrap_err();

        match result {
            PromptAssemblyError::TemplateSyntaxNotAllowed { token, .. } => {
                assert_eq!(token, "{%");
            }
            other => panic!("Expected TemplateSyntaxNotAllowed, got {:?}", other),
        }
    }

    #[test]
    fn render_template_fails_on_undefined_variable() {
        let template = "missing: {{missing}}";
        let context = PromptContext::new();

        let result = render_template(template, &context, "inline").unwrap_err();

        match result {
            PromptAssemblyError::TemplateRenderError { .. } => {}
            other => panic!("Expected TemplateRenderError, got {:?}", other),
        }
    }

    #[test]
    fn test_assemble_prompt_path_traversal() {
        let mock_fs = MockPromptFs::new();
        let jules_path = Path::new(".jules");

        mock_fs.add_file(
            ".jules/roles/planners/prompt_assembly.yml",
            r#"
schema_version: 1
layer: planners
runtime_context: {}
includes:
  - title: "Malicious"
    path: "../secret.txt"
"#,
        );
        mock_fs.add_file(".jules/roles/planners/prompt.yml", "role: planners\nlayer: planners");

        let result = assemble_prompt(jules_path, Layer::Planners, &PromptContext::new(), &mock_fs);

        assert!(result.is_err());
        match result.unwrap_err() {
            PromptAssemblyError::PathTraversalDetected { path } => {
                assert_eq!(path, "../secret.txt");
            }
            other => panic!("Expected PathTraversalDetected, got {:?}", other),
        }
    }

    #[test]
    fn test_assemble_prompt_absolute_path_traversal() {
        let mock_fs = MockPromptFs::new();
        let jules_path = Path::new(".jules");

        mock_fs.add_file(
            ".jules/roles/planners/prompt_assembly.yml",
            r#"
schema_version: 1
layer: planners
runtime_context: {}
includes:
  - title: "Malicious Absolute"
    path: "/etc/passwd"
"#,
        );
        mock_fs.add_file(".jules/roles/planners/prompt.yml", "role: planners\nlayer: planners");

        let result = assemble_prompt(jules_path, Layer::Planners, &PromptContext::new(), &mock_fs);

        assert!(result.is_err());
        match result.unwrap_err() {
            PromptAssemblyError::PathTraversalDetected { path } => {
                assert_eq!(path, "/etc/passwd");
            }
            other => panic!("Expected PathTraversalDetected, got {:?}", other),
        }
    }
}
