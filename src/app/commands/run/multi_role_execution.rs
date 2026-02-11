use std::path::Path;

use crate::domain::workspace::paths::{jlo, jules};
use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{AutomationMode, JulesClient, SessionRequest};

pub(crate) fn print_role_preview(
    jules_path: &Path,
    layer: Layer,
    role: &RoleId,
    starting_branch: &str,
) {
    println!("=== Prompt Preview: {} ===", layer.display_name());
    println!("Starting branch: {}", starting_branch);
    println!("Role: {}\n", role);

    let root = jules_path.parent().unwrap_or(Path::new("."));
    let role_yml_path = jlo::role_yml(root, layer, role.as_str());

    if !role_yml_path.exists() {
        println!("  ⚠️  role.yml not found at {}\n", role_yml_path.display());
        return;
    }

    let contracts_path = jules::contracts(jules_path, layer);
    if contracts_path.exists() {
        println!("  Contracts: {}", contracts_path.display());
    }
    println!("  Role config: {}", role_yml_path.display());
}

pub(crate) fn validate_role_exists(
    jules_path: &Path,
    layer: Layer,
    role: &str,
) -> Result<(), AppError> {
    let root = jules_path.parent().unwrap_or(Path::new("."));
    let role_yml_path = jlo::role_yml(root, layer, role);

    if !role_yml_path.exists() {
        return Err(AppError::RoleNotFound(format!(
            "{}/{} (role.yml not found)",
            layer.dir_name(),
            role
        )));
    }

    Ok(())
}

pub(crate) fn dispatch_session<C: JulesClient>(
    layer: Layer,
    role: &RoleId,
    prompt: String,
    source: &str,
    starting_branch: &str,
    client: &C,
) -> Result<String, AppError> {
    println!("Executing {} / {}...", layer.dir_name(), role);

    let request = SessionRequest {
        prompt,
        source: source.to_string(),
        starting_branch: starting_branch.to_string(),
        require_plan_approval: false,
        automation_mode: AutomationMode::AutoCreatePr,
    };

    let response = client.create_session(request)?;
    println!("  ✅ Session created: {}", response.session_id);

    Ok(response.session_id)
}
