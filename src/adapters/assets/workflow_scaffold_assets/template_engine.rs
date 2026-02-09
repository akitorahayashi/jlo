use minijinja::{Environment, Value};

use crate::domain::AppError;

use super::asset_collect::AssetSourceFile;

fn gha_expr(expr: &str) -> String {
    format!("${{{{ {} }}}}", expr)
}

fn gha_raw(expr: &str) -> String {
    format!("${{{{ {} }}}}", expr)
}

pub fn build_template_environment(
    sources: &[AssetSourceFile],
) -> Result<Environment<'_>, AppError> {
    let mut env = Environment::new();
    env.set_keep_trailing_newline(true);

    env.add_function("gha_expr", |expr: &str| -> String { gha_expr(expr) });
    env.add_function("gha_raw", |expr: &str| -> String { gha_raw(expr) });

    for source in sources.iter().filter(|source| source.is_template()) {
        env.add_template(source.template_name(), source.content.as_str()).map_err(|e| {
            AppError::InternalError(format!(
                "Failed to register template '{}': {}",
                source.template_name(),
                e
            ))
        })?;
    }

    Ok(env)
}

pub fn render_template_by_name(
    env: &Environment<'_>,
    template_name: &str,
    ctx: &Value,
) -> Result<String, AppError> {
    let template = env.get_template(template_name).map_err(|e| {
        AppError::InternalError(format!("Failed to load template '{}': {}", template_name, e))
    })?;

    template.render(ctx).map_err(|e| {
        AppError::InternalError(format!("Failed to render template '{}': {}", template_name, e))
    })
}
