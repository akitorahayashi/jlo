use std::path::Path;

use crate::domain::repository::paths::{jlo, jules};
use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{AutomationMode, JulesClient, RepositoryFilesystem, SessionRequest};

pub fn print_role_preview<W: RepositoryFilesystem + ?Sized>(
    jules_path: &Path,
    layer: Layer,
    role: &RoleId,
    starting_branch: &str,
    repository: &W,
) {
    println!("=== Prompt Preview: {} ===", layer.display_name());
    println!("Starting branch: {}", starting_branch);
    println!("Role: {}\n", role);

    let root = jules_path.parent().unwrap_or(Path::new("."));
    let role_yml_path = jlo::role_yml(root, layer, role.as_str());

    if !repository.file_exists(&role_yml_path.to_string_lossy()) {
        println!("  ⚠️  role.yml not found at {}\n", role_yml_path.display());
        return;
    }

    let contracts_path = jules::contracts(jules_path, layer);
    if repository.file_exists(&contracts_path.to_string_lossy()) {
        println!("  Contracts: {}", contracts_path.display());
    }
    println!("  Role config: {}", role_yml_path.display());
}

pub fn validate_role_exists<W: RepositoryFilesystem + ?Sized>(
    jules_path: &Path,
    layer: Layer,
    role: &str,
    repository: &W,
) -> Result<(), AppError> {
    let root = jules_path.parent().unwrap_or(Path::new("."));
    let role_yml_path = jlo::role_yml(root, layer, role);

    if !repository.file_exists(&role_yml_path.to_string_lossy()) {
        return Err(AppError::RoleNotFound(format!(
            "{}/{} (role.yml not found)",
            layer.dir_name(),
            role
        )));
    }

    Ok(())
}

pub fn dispatch_session<C: JulesClient + ?Sized, S: Into<String>>(
    layer: Layer,
    role: &RoleId,
    prompt: String,
    source: &str,
    starting_branch: S,
    client: &C,
) -> Result<String, AppError> {
    println!("Executing {} / {}...", layer.dir_name(), role);

    let request = SessionRequest {
        prompt,
        source: source.to_string(),
        starting_branch: starting_branch.into(),
        require_plan_approval: false,
        automation_mode: AutomationMode::AutoCreatePr,
    };

    let response = client.create_session(request)?;
    println!("  ✅ Session created: {}", response.session_id);

    Ok(response.session_id)
}
