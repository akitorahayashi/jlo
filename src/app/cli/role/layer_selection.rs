use crate::domain::{AppError, Layer};
use dialoguer::Select;

pub(super) fn parse_multi_role_layer(value: &str) -> Result<Layer, AppError> {
    let layer =
        Layer::from_dir_name(value).ok_or(AppError::InvalidLayer { name: value.to_string() })?;
    if layer.is_single_role() {
        return Err(AppError::SingleRoleLayerTemplate(layer.dir_name().to_string()));
    }
    Ok(layer)
}

pub(super) fn prompt_multi_role_layer() -> Result<Option<Layer>, AppError> {
    let layers: Vec<Layer> =
        Layer::ALL.into_iter().filter(|layer| !layer.is_single_role()).collect();
    if layers.is_empty() {
        return Err(AppError::Validation("No multi-role layers available".to_string()));
    }

    let items: Vec<String> =
        layers.iter().map(|layer| layer.display_name().to_lowercase()).collect();
    let selection = Select::new()
        .with_prompt("Select layer")
        .items(&items)
        .default(0)
        .interact_opt()
        .map_err(|err| AppError::Validation(format!("Failed to select layer: {}", err)))?;

    Ok(selection.map(|index| layers[index]))
}
