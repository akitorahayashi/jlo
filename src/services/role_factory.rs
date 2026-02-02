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
        _workstream: Option<&str>,
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

        // Generate role.yml with ROLE_NAME substitution
        let mut role_yaml = templates.generate_role_yaml(role_name, layer);
        role_yaml = role_yaml.replace("ROLE_NAME", role_name);

        workspace.scaffold_role_in_layer(layer, &role_id, &role_yaml)?;

        Ok(())
    }
}
