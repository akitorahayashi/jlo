use minijinja::{Environment, UndefinedBehavior};
use std::sync::OnceLock;

use crate::domain::prompt::{PromptAssemblyError, PromptContext, TemplateRenderer};

/// Template renderer using Minijinja.
pub struct MinijinjaTemplateRenderer;

impl MinijinjaTemplateRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl TemplateRenderer for MinijinjaTemplateRenderer {
    fn render(
        &self,
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
}

static ENV: OnceLock<Environment<'static>> = OnceLock::new();

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
