//! Setup command implementation.

use std::path::PathBuf;

use crate::adapters::assets::component_catalog_embedded::EmbeddedComponentCatalog;
use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
use crate::domain::AppError;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum SetupCommands {
    /// Generate install.sh and env.toml from tools.yml
    #[clap(visible_alias = "g")]
    Gen {
        /// Project directory containing .jules/setup/ (defaults to current directory)
        path: Option<PathBuf>,
    },
    /// List available components
    #[clap(visible_alias = "ls")]
    List {
        /// Show detailed info for a specific component
        #[arg(long)]
        detail: Option<String>,
    },
}

pub fn run_setup_gen(path: Option<PathBuf>) -> Result<(), AppError> {
    let store = if let Some(p) = path {
        FilesystemWorkspaceStore::new(p)
    } else {
        FilesystemWorkspaceStore::current()?
    };
    let catalog = EmbeddedComponentCatalog::new()?;

    let components = crate::app::commands::setup::generate(&store, &catalog)?;
    println!("✅ Generated install.sh with {} component(s)", components.len());
    for (i, name) in components.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }
    Ok(())
}

pub fn run_setup_list(detail: Option<String>) -> Result<(), AppError> {
    let catalog = EmbeddedComponentCatalog::new()?;

    if let Some(component) = detail {
        let info = crate::app::commands::setup::list_detail(&catalog, &component)?;
        println!("{}: {}", info.name, info.summary);
        if !info.dependencies.is_empty() {
            println!("\nDependencies:");
            for dep in &info.dependencies {
                println!("  • {}", dep);
            }
        }
        if !info.env_vars.is_empty() {
            println!("\nEnvironment Variables:");
            for env in &info.env_vars {
                let default_str =
                    env.default.as_ref().map(|d| format!(" (default: {})", d)).unwrap_or_default();
                println!("  • {}{}", env.name, default_str);
                if !env.description.is_empty() {
                    println!("    {}", env.description);
                }
            }
        }
        println!("\nInstall Script:");
        println!("{}", info.script_content);
    } else {
        let components = crate::app::commands::setup::list(&catalog)?;
        println!("Available components:");
        for comp in components {
            println!("  {} - {}", comp.name, comp.summary);
        }
    }
    Ok(())
}
