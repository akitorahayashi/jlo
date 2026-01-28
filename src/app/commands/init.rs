use crate::app::AppContext;
use crate::domain::AppError;
use crate::ports::{ClipboardWriter, ComponentCatalog, RoleTemplateStore, WorkspaceStore};
use crate::services::EmbeddedCatalog;

const SETUP_GITIGNORE: &str = r#"# Ignore environment configuration with secrets
env.toml
"#;

/// Generate tools.yml template dynamically from the catalog.
fn generate_tools_template() -> Result<String, AppError> {
    let catalog = EmbeddedCatalog::new()?;
    let mut template = String::from(
        "# jlo setup configuration\n\
         # List the tools you want to install\n\
         \n\
         tools:\n",
    );

    for name in catalog.names() {
        template.push_str(&format!("  # - {}\n", name));
    }

    Ok(template)
}

/// Execute the init command.
///
/// Creates both the `.jules/` workspace and `.jules/setup/` directory.
pub fn execute<W, R, C>(ctx: &AppContext<W, R, C>) -> Result<(), AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
    C: ClipboardWriter,
{
    if ctx.workspace().exists() {
        return Err(AppError::WorkspaceExists);
    }

    let scaffold_files = ctx.templates().scaffold_files();
    ctx.workspace().create_structure(&scaffold_files)?;

    ctx.workspace().write_version(env!("CARGO_PKG_VERSION"))?;

    // Create setup directory structure
    let setup_dir = ctx.workspace().jules_path().join("setup");
    std::fs::create_dir_all(&setup_dir)?;
    std::fs::write(setup_dir.join("tools.yml"), generate_tools_template()?)?;
    std::fs::write(setup_dir.join(".gitignore"), SETUP_GITIGNORE)?;

    Ok(())
}
