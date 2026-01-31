use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{RoleTemplateStore, WorkspaceStore};

/// Service for creating new agent roles.
///
/// Encapsulates the business logic for generating role files,
/// applying templates, and handling variable substitution (e.g. ROLE_NAME).
pub struct RoleFactory;

impl RoleFactory {
    /// Create a new role in the specified layer.
    pub fn create_role<W, T>(
        workspace: &W,
        templates: &T,
        layer: Layer,
        role_name: &str,
        workstream: Option<&str>,
    ) -> Result<(), AppError>
    where
        W: WorkspaceStore,
        T: RoleTemplateStore,
    {
        let role_id = RoleId::new(role_name)?;

        if workspace.role_exists_in_layer(layer, &role_id) {
            return Err(AppError::RoleExists {
                role: role_name.to_string(),
                layer: layer.dir_name().to_string(),
            });
        }

        let role_yaml = templates.generate_role_yaml(role_name, layer);
        let mut prompt_yaml = templates.generate_prompt_yaml_template(role_name, layer);

        // Domain logic: Apply substitutions
        prompt_yaml = prompt_yaml.replace("ROLE_NAME", role_name);

        if let Some(ws) = workstream {
            let placeholder = "workstream: generic";
            if prompt_yaml.contains(placeholder) {
                prompt_yaml = prompt_yaml.replace(placeholder, &format!("workstream: {}", ws));
            } else {
                return Err(AppError::config_error(
                    "Prompt template missing workstream placeholder; cannot apply workstream.",
                ));
            }
        }

        let has_notes = matches!(layer, Layer::Observers);
        workspace.scaffold_role_in_layer(
            layer,
            &role_id,
            &role_yaml,
            Some(&prompt_yaml),
            has_notes,
        )?;

        Ok(())
    }
}
