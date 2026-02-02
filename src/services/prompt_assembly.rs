//! Prompt assembly service.
//!
//! Loads `prompt_assembly.yml` from the workspace and assembles the final prompt
//! by reading the base `prompt.yml`, substituting placeholders, and concatenating
//! included files with section headers.

use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use minijinja::{Environment, UndefinedBehavior};

use crate::domain::{
    AssembledPrompt, Layer, PromptAssemblyError, PromptAssemblySpec, PromptContext,
};

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
) -> Result<AssembledPrompt, PromptAssemblyError> {
    let layer_dir = jules_path.join("roles").join(layer.dir_name());
    let root = jules_path.parent().unwrap_or(Path::new("."));

    // Load prompt_assembly.yml
    let assembly_path = layer_dir.join("prompt_assembly.yml");
    let spec = load_assembly_spec(&assembly_path)?;

    // Validate required context variables
    validate_context(&spec, context)?;

    // Load base prompt.yml
    let prompt_path = layer_dir.join("prompt.yml");
    let base_prompt = load_prompt(&prompt_path, context)?;

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
        let full_path = root.join(&resolved_path);

        // Auto-initialize from schema if missing
        if !full_path.exists()
            && let Some(file_name) = Path::new(&resolved_path).file_name()
        {
            let schema_path = layer_dir.join("schemas").join(file_name);
            if schema_path.exists() {
                if let Some(parent) = full_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::copy(&schema_path, &full_path);
            }
        }

        if full_path.exists() {
            match fs::read_to_string(&full_path) {
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

/// Assemble a prompt for an issue-driven layer (planners, implementers).
///
/// This appends the issue content to the base assembled prompt.
#[allow(dead_code)]
pub fn assemble_with_issue(
    jules_path: &Path,
    layer: Layer,
    issue_content: &str,
) -> Result<AssembledPrompt, PromptAssemblyError> {
    let mut result = assemble_prompt(jules_path, layer, &PromptContext::new())?;

    result.content.push_str(&format!("\n---\n# Issue\n{}", issue_content));
    result.included_files.push("(issue content embedded)".to_string());

    Ok(result)
}

/// Load and parse the prompt assembly spec from a file.
fn load_assembly_spec(path: &Path) -> Result<PromptAssemblySpec, PromptAssemblyError> {
    if !path.exists() {
        return Err(PromptAssemblyError::AssemblySpecNotFound(path.display().to_string()));
    }

    let content =
        fs::read_to_string(path).map_err(|err| PromptAssemblyError::InvalidAssemblySpec {
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
fn load_prompt(path: &Path, context: &PromptContext) -> Result<String, PromptAssemblyError> {
    if !path.exists() {
        return Err(PromptAssemblyError::PromptNotFound(path.display().to_string()));
    }

    let content = fs::read_to_string(path).map_err(|err| PromptAssemblyError::PromptReadError {
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
    use tempfile::tempdir;

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
        let result = assemble_prompt(&jules_path, Layer::Planners, &PromptContext::new());

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

        let result = assemble_prompt(&jules_path, Layer::Observers, &context);

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

        let result = assemble_prompt(&jules_path, Layer::Observers, &context);

        assert!(result.is_err());
        match result.unwrap_err() {
            PromptAssemblyError::MissingContextVariable { variable, .. } => {
                assert_eq!(variable, "role");
            }
            other => panic!("Expected MissingContextVariable, got {:?}", other),
        }
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
}
