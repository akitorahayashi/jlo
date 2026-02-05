//! Prompt assembly service.
//!
//! Loads `prompt_assembly.yml` from the workspace and assembles the final prompt
//! by reading the base `prompt.yml`, substituting placeholders, and concatenating
//! included files with section headers.

use std::path::{Component, Path};
use std::sync::OnceLock;

use minijinja::{Environment, UndefinedBehavior};

use crate::domain::{
    AppError, AssembledPrompt, JULES_DIR, Layer, PromptAssemblyError, PromptAssemblySpec,
    PromptContext,
};
use crate::ports::WorkspaceStore;

/// Assemble a prompt for the given layer using the prompt assembly spec.
///
/// For multi-role layers (observers, deciders), the context must include
/// `workstream` and `role` variables. For single-role layers, the context
/// may be empty.
///
/// For issue-driven layers (planners, implementers), use `assemble_with_issue`
/// to append issue content to the assembled prompt.
pub fn assemble_prompt(
    workspace: &impl WorkspaceStore,
    layer: Layer,
    context: &PromptContext,
) -> Result<AssembledPrompt, PromptAssemblyError> {
    let layer_dir = Path::new(JULES_DIR).join("roles").join(layer.dir_name());

    // Load prompt_assembly.yml
    let assembly_path = layer_dir.join("prompt_assembly.yml");
    let assembly_path_str = assembly_path.to_string_lossy().to_string();
    let spec = load_assembly_spec(&assembly_path_str, workspace)?;

    // Validate required context variables
    validate_context(&spec, context)?;

    // Load base prompt.yml
    let prompt_path = layer_dir.join("prompt.yml");
    let prompt_path_str = prompt_path.to_string_lossy().to_string();
    let base_prompt = load_prompt(&prompt_path_str, context, workspace)?;

    // Assemble includes
    let mut parts = vec![base_prompt];
    let mut included_files = vec![prompt_path_str.clone()];
    let mut skipped_files = Vec::new();

    for include in &spec.includes {
        let resolved_path = render_template(
            &include.path,
            context,
            &format!("prompt_assembly include path ({})", include.title),
        )?;
        validate_safe_path(&resolved_path)?;

        // Path is relative to workspace root
        let full_path_str = resolved_path.clone();

        // Auto-initialize from schema if missing
        // Check if file exists in workspace
        if !workspace.path_exists(&full_path_str)
            && let Some(file_name) = Path::new(&resolved_path).file_name()
        {
            let schema_path = layer_dir.join("schemas").join(file_name);
            let schema_path_str = schema_path.to_string_lossy().to_string();

            if workspace.path_exists(&schema_path_str) {
                // We need to ensure directory exists.
                // WorkspaceStore::write_file usually creates dirs, but copy_file might too?
                // WorkspaceStore::copy_file docs say generic op.
                // Let's assume copy_file creates dirs or we call create_dir_all if needed.
                // But we can't easily get parent of string without Path.
                // Let's trust copy_file or implement logic.
                // In FilesystemWorkspaceStore::copy_file, it creates parents.
                let _ = workspace.copy_file(&schema_path_str, &full_path_str);
            }
        }

        if workspace.path_exists(&full_path_str) {
            match workspace.read_file(&full_path_str) {
                Ok(content) => {
                    parts.push(format!("\n---\n# {}\n{}", include.title, content));
                    included_files.push(resolved_path);
                }
                Err(AppError::Io(err)) => {
                    if include.optional {
                        skipped_files.push(format!("{} (read error: {})", resolved_path, err));
                    } else {
                        return Err(PromptAssemblyError::IncludeReadError {
                            path: resolved_path,
                            reason: err.to_string(),
                        });
                    }
                }
                Err(err) => {
                    // Any other error
                    return Err(PromptAssemblyError::IncludeReadError {
                        path: resolved_path,
                        reason: err.to_string(),
                    });
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
    workspace: &impl WorkspaceStore,
    layer: Layer,
    issue_content: &str,
) -> Result<AssembledPrompt, PromptAssemblyError> {
    let mut result = assemble_prompt(workspace, layer, &PromptContext::new())?;

    result.content.push_str(&format!("\n---\n# Issue\n{}", issue_content));
    result.included_files.push("(issue content embedded)".to_string());

    Ok(result)
}

/// Load and parse the prompt assembly spec from a file.
fn load_assembly_spec(
    path: &str,
    workspace: &impl WorkspaceStore,
) -> Result<PromptAssemblySpec, PromptAssemblyError> {
    if !workspace.path_exists(path) {
        return Err(PromptAssemblyError::AssemblySpecNotFound(path.to_string()));
    }

    let content = workspace.read_file(path).map_err(|err| {
        PromptAssemblyError::InvalidAssemblySpec { path: path.to_string(), reason: err.to_string() }
    })?;

    serde_yaml::from_str(&content).map_err(|err| PromptAssemblyError::InvalidAssemblySpec {
        path: path.to_string(),
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
    path: &str,
    context: &PromptContext,
    workspace: &impl WorkspaceStore,
) -> Result<String, PromptAssemblyError> {
    if !workspace.path_exists(path) {
        return Err(PromptAssemblyError::PromptNotFound(path.to_string()));
    }

    let content = workspace.read_file(path).map_err(|err| {
        PromptAssemblyError::PromptReadError { path: path.to_string(), reason: err.to_string() }
    })?;

    render_template(&content, context, path)
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
    use crate::testing::mock_workspace_store::MockWorkspaceStore;

    // Helper to setup mock workspace
    fn setup_mock_workspace(layer: &str, single_role: bool) -> MockWorkspaceStore {
        let mock_store = MockWorkspaceStore::new();
        let layer_path = format!(".jules/roles/{}", layer);

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

        mock_store.write_file(&format!("{}/prompt_assembly.yml", layer_path), &assembly).unwrap();

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
        mock_store.write_file(&format!("{}/prompt.yml", layer_path), &prompt).unwrap();

        mock_store
            .write_file(&format!("{}/contracts.yml", layer_path), "layer: test\nconstraints: []")
            .unwrap();

        if !single_role {
            let role_path = format!("{}/roles/test_role", layer_path);
            mock_store
                .write_file(&format!("{}/role.yml", role_path), "role: test_role\nfocus: testing")
                .unwrap();
        }

        mock_store
    }

    #[test]
    fn assemble_single_role_prompt() {
        let mock_store = setup_mock_workspace("planners", true);

        let result = assemble_prompt(&mock_store, Layer::Planners, &PromptContext::new());

        assert!(result.is_ok());
        let assembled = result.unwrap();
        assert!(assembled.content.contains("role: planners"));
        assert!(assembled.content.contains("# Layer Contracts"));
        assert!(assembled.included_files.len() >= 2);
    }

    #[test]
    fn assemble_multi_role_prompt() {
        let mock_store = setup_mock_workspace("observers", false);

        let context =
            PromptContext::new().with_var("workstream", "generic").with_var("role", "test_role");

        let result = assemble_prompt(&mock_store, Layer::Observers, &context);

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
        let mock_store = setup_mock_workspace("observers", false);

        // Missing 'role' variable
        let context = PromptContext::new().with_var("workstream", "generic");

        let result = assemble_prompt(&mock_store, Layer::Observers, &context);

        assert!(result.is_err());
        match result.unwrap_err() {
            PromptAssemblyError::MissingContextVariable { variable, .. } => {
                assert_eq!(variable, "role");
            }
            other => panic!("Expected MissingContextVariable, got {:?}", other),
        }
    }

    #[test]
    fn test_assemble_prompt_mock_fs() {
        let mock_store = MockWorkspaceStore::new();

        // Setup mock files for Planners layer
        mock_store
            .write_file(
                ".jules/roles/planners/prompt_assembly.yml",
                r#"
schema_version: 1
layer: planners
runtime_context: {}
includes:
  - title: "Contracts"
    path: ".jules/roles/planners/contracts.yml"
"#,
            )
            .unwrap();
        mock_store
            .write_file(".jules/roles/planners/prompt.yml", "role: planners\nlayer: planners")
            .unwrap();
        mock_store
            .write_file(".jules/roles/planners/contracts.yml", "layer: planners\nconstraints: []")
            .unwrap();

        let result = assemble_prompt(&mock_store, Layer::Planners, &PromptContext::new());

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
        let mock_store = MockWorkspaceStore::new();

        mock_store
            .write_file(
                ".jules/roles/planners/prompt_assembly.yml",
                r#"
schema_version: 1
layer: planners
runtime_context: {}
includes:
  - title: "Malicious"
    path: "../secret.txt"
"#,
            )
            .unwrap();
        mock_store
            .write_file(".jules/roles/planners/prompt.yml", "role: planners\nlayer: planners")
            .unwrap();

        let result = assemble_prompt(&mock_store, Layer::Planners, &PromptContext::new());

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
        let mock_store = MockWorkspaceStore::new();

        mock_store
            .write_file(
                ".jules/roles/planners/prompt_assembly.yml",
                r#"
schema_version: 1
layer: planners
runtime_context: {}
includes:
  - title: "Malicious Absolute"
    path: "/etc/passwd"
"#,
            )
            .unwrap();
        mock_store
            .write_file(".jules/roles/planners/prompt.yml", "role: planners\nlayer: planners")
            .unwrap();

        let result = assemble_prompt(&mock_store, Layer::Planners, &PromptContext::new());

        assert!(result.is_err());
        match result.unwrap_err() {
            PromptAssemblyError::PathTraversalDetected { path } => {
                assert_eq!(path, "/etc/passwd");
            }
            other => panic!("Expected PathTraversalDetected, got {:?}", other),
        }
    }
}
