use super::layer_selection::{parse_multi_role_layer, prompt_multi_role_layer};
use crate::domain::AppError;
use dialoguer::{Error as DialoguerError, Input};
use std::io::ErrorKind;

pub fn run(layer: Option<String>, role: Option<String>) -> Result<(), AppError> {
    let Some((layer, role)) = resolve_inputs(layer, role)? else {
        return Ok(());
    };
    let outcome = crate::app::api::role_create(&layer, &role)?;
    println!("✅ Created new {} at {}/", outcome.entity_type(), outcome.display_path());
    Ok(())
}

fn resolve_inputs(
    layer: Option<String>,
    role: Option<String>,
) -> Result<Option<(String, String)>, AppError> {
    let layer_enum = match layer {
        Some(value) => parse_multi_role_layer(&value)?,
        None => match prompt_multi_role_layer()? {
            Some(value) => value,
            None => return Ok(None),
        },
    };

    let role_value = match role {
        Some(value) => value,
        None => match prompt_role_name()? {
            Some(value) => value,
            None => return Ok(None),
        },
    };

    Ok(Some((layer_enum.dir_name().to_string(), role_value)))
}

fn prompt_role_name() -> Result<Option<String>, AppError> {
    match Input::new().with_prompt("Role name").interact_text() {
        Ok(value) => Ok(Some(value)),
        Err(DialoguerError::IO(err)) if err.kind() == ErrorKind::Interrupted => Ok(None),
        Err(err) => Err(AppError::Validation(format!("Failed to read role name: {}", err))),
    }
}
