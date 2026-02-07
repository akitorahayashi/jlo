use super::prompt_assembly::{PromptAssemblyError, PromptContext};

/// Trait for rendering templates.
///
/// This abstraction allows swapping out the template engine (e.g. minijinja)
/// and keeping infrastructure details out of the domain layer.
pub trait TemplateRenderer {
    /// Render a template string with the given context.
    ///
    /// # Arguments
    /// * `template` - The template string to render.
    /// * `context` - The context variables to use for rendering.
    /// * `template_name` - A name for the template (for error reporting).
    fn render(
        &self,
        template: &str,
        context: &PromptContext,
        template_name: &str,
    ) -> Result<String, PromptAssemblyError>;
}
