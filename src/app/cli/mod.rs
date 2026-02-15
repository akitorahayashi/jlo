//! CLI Adapter.

mod deinit;
mod doctor;
mod init;
mod run;
mod setup;
mod workflow;

use crate::domain::{AppError, BuiltinRoleEntry, Layer};
use clap::{Parser, Subcommand};
use dialoguer::{Error as DialoguerError, Input, Select};
use std::collections::BTreeMap;
use std::io::ErrorKind;

#[derive(Parser)]
#[command(name = "jlo")]
#[command(version)]
#[command(
    about = "Deploy and manage .jules/ repository scaffolding",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize .jlo/ control plane and install workflow scaffold
    #[clap(visible_alias = "i")]
    Init {
        /// Install the GitHub-hosted runner workflow scaffold
        #[arg(
            short = 'r',
            long,
            conflicts_with = "self_hosted",
            required_unless_present = "self_hosted"
        )]
        remote: bool,
        /// Install the self-hosted runner workflow scaffold
        #[arg(short = 's', long, conflicts_with = "remote", required_unless_present = "remote")]
        self_hosted: bool,
    },
    /// Advance .jlo/ control-plane version pin
    #[clap(visible_alias = "u")]
    Update {
        /// Show planned changes without applying
        #[arg(long, conflicts_with = "cli")]
        prompt_preview: bool,
        /// Update the jlo CLI binary from upstream releases
        #[arg(short = 'c', long, conflicts_with = "prompt_preview")]
        cli: bool,
    },
    /// Create a new role under .jlo/
    #[clap(visible_alias = "cr")]
    Create {
        /// Layer (observers, innovators)
        layer: Option<String>,
        /// Name for the new role
        role: Option<String>,
    },
    /// Add a built-in role under .jlo/
    #[clap(visible_aliases = ["a", "ad"])]
    Add {
        /// Layer (observers, innovators)
        layer: Option<String>,
        /// Built-in role name(s)
        roles: Vec<String>,
    },
    /// Setup compiler commands
    #[clap(visible_alias = "s")]
    Setup {
        #[command(subcommand)]
        command: setup::SetupCommands,
    },
    /// Execute Jules agents
    #[clap(visible_alias = "r")]
    Run {
        #[command(subcommand)]
        layer: run::RunLayer,
    },
    /// Workflow orchestration primitives for GitHub Actions
    #[clap(visible_alias = "wf")]
    Workflow {
        #[command(subcommand)]
        command: workflow::WorkflowCommands,
    },
    /// Validate .jules/ structure and content
    Doctor {
        /// Treat warnings as failures
        #[arg(long)]
        strict: bool,
    },
    /// Remove jlo-managed assets (branch + workflows)
    Deinit,
}

/// Entry point for the CLI.
pub fn run() {
    let cli = Cli::parse();

    let result: Result<i32, AppError> = match cli.command {
        Commands::Init { remote, self_hosted } => init::run_init(remote, self_hosted).map(|_| 0),
        Commands::Update { prompt_preview, cli } => run_update(prompt_preview, cli).map(|_| 0),
        Commands::Create { layer, role } => run_create(layer, role).map(|_| 0),
        Commands::Add { layer, roles } => run_add(layer, roles).map(|_| 0),
        Commands::Setup { command } => match command {
            setup::SetupCommands::Gen { path } => setup::run_setup_gen(path).map(|_| 0),
            setup::SetupCommands::List { detail } => setup::run_setup_list(detail).map(|_| 0),
        },
        Commands::Run { layer } => run::run_agents(layer).map(|_| 0),
        Commands::Workflow { command } => workflow::run_workflow(command).map(|_| 0),
        Commands::Doctor { strict } => doctor::run_doctor(strict),
        Commands::Deinit => deinit::run_deinit().map(|_| 0),
    };

    match result {
        Ok(exit_code) => {
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_update(prompt_preview: bool, cli: bool) -> Result<(), AppError> {
    if cli {
        let result = crate::app::api::update_cli()?;
        if result.upgraded {
            println!("✅ Updated jlo CLI from {} to {}", result.current_version, result.latest_tag);
        } else {
            println!(
                "✅ jlo CLI is already up to date (current: {}, latest: {})",
                result.current_version, result.latest_tag
            );
        }
        return Ok(());
    }

    let result = crate::app::api::update(prompt_preview)?;

    if !result.prompt_preview {
        if !result.warnings.is_empty() {
            println!("⚠️  Update warnings:");
            for warning in &result.warnings {
                println!("  • {}", warning);
            }
        }

        if result.created.is_empty()
            && result.updated.is_empty()
            && !result.workflow_refreshed
            && result.previous_version == env!("CARGO_PKG_VERSION")
        {
            println!("✅ Repository already up to date");
        } else {
            println!("✅ Updated repository to version {}", env!("CARGO_PKG_VERSION"));
            if !result.created.is_empty() {
                println!("  Created {} file(s)", result.created.len());
            }
            if !result.updated.is_empty() {
                println!("  Refreshed {} managed default file(s)", result.updated.len());
            }
            if result.workflow_refreshed {
                println!("  Refreshed workflow scaffold");
            }
        }
    }

    Ok(())
}

fn run_create(layer: Option<String>, role: Option<String>) -> Result<(), AppError> {
    let Some((layer, role)) = resolve_create_inputs(layer, role)? else {
        return Ok(());
    };
    let outcome = crate::app::api::create_role(&layer, &role)?;

    println!("✅ Created new {} at {}/", outcome.entity_type(), outcome.display_path());
    Ok(())
}

fn run_add(layer: Option<String>, roles: Vec<String>) -> Result<(), AppError> {
    let Some((layer, roles)) = resolve_add_inputs(layer, roles)? else {
        return Ok(());
    };

    for role in roles {
        let outcome = crate::app::api::add_role(&layer, &role)?;
        println!("✅ Added new {} at {}/", outcome.entity_type(), outcome.display_path());
    }
    Ok(())
}

fn resolve_create_inputs(
    layer: Option<String>,
    role: Option<String>,
) -> Result<Option<(String, String)>, AppError> {
    let layer_enum = match layer {
        Some(value) => {
            let l = Layer::from_dir_name(&value).ok_or(AppError::InvalidLayer { name: value })?;
            if l.is_single_role() {
                return Err(AppError::SingleRoleLayerTemplate(l.dir_name().to_string()));
            }
            l
        }
        None => match prompt_multi_role_layer()? {
            Some(value) => {
                Layer::from_dir_name(&value).ok_or(AppError::InvalidLayer { name: value })?
            }
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

fn resolve_add_inputs(
    layer: Option<String>,
    roles: Vec<String>,
) -> Result<Option<(String, Vec<String>)>, AppError> {
    if !roles.is_empty() {
        let layer_enum = match layer {
            Some(value) => {
                let l =
                    Layer::from_dir_name(&value).ok_or(AppError::InvalidLayer { name: value })?;
                if l.is_single_role() {
                    return Err(AppError::SingleRoleLayerTemplate(l.dir_name().to_string()));
                }
                l
            }
            None => match prompt_multi_role_layer()? {
                Some(value) => {
                    Layer::from_dir_name(&value).ok_or(AppError::InvalidLayer { name: value })?
                }
                None => return Ok(None),
            },
        };
        return Ok(Some((layer_enum.dir_name().to_string(), roles)));
    }

    let catalog = crate::app::api::builtin_role_catalog()?;

    if let Some(value) = layer {
        let layer_enum =
            Layer::from_dir_name(&value).ok_or(AppError::InvalidLayer { name: value })?;
        if layer_enum.is_single_role() {
            return Err(AppError::SingleRoleLayerTemplate(layer_enum.dir_name().to_string()));
        }

        return match prompt_builtin_role(&catalog, layer_enum, false)? {
            BuiltinRoleSelection::Selected(role) => {
                Ok(Some((layer_enum.dir_name().to_string(), vec![role])))
            }
            BuiltinRoleSelection::Cancel => Ok(None),
            BuiltinRoleSelection::BackToLayer => unreachable!("layer is fixed"),
        };
    }

    loop {
        let Some(selected_layer) = prompt_multi_role_layer()? else {
            return Ok(None);
        };
        let layer_enum = Layer::from_dir_name(&selected_layer)
            .ok_or(AppError::InvalidLayer { name: selected_layer })?;
        match prompt_builtin_role(&catalog, layer_enum, true)? {
            BuiltinRoleSelection::Selected(role) => {
                return Ok(Some((layer_enum.dir_name().to_string(), vec![role])));
            }
            BuiltinRoleSelection::BackToLayer => continue,
            BuiltinRoleSelection::Cancel => return Ok(None),
        }
    }
}

fn prompt_multi_role_layer() -> Result<Option<String>, AppError> {
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

    Ok(selection.map(|index| layers[index].dir_name().to_string()))
}

enum BuiltinRoleSelection {
    Selected(String),
    BackToLayer,
    Cancel,
}

const MENU_BACK_OPTION: &str = "[back]";

fn prompt_builtin_role(
    catalog: &[BuiltinRoleEntry],
    layer: Layer,
    allow_layer_back: bool,
) -> Result<BuiltinRoleSelection, AppError> {
    let entries_by_category = catalog.iter().filter(|entry| entry.layer == layer).fold(
        BTreeMap::<&str, Vec<&BuiltinRoleEntry>>::new(),
        |mut map, entry| {
            map.entry(entry.category.as_str()).or_default().push(entry);
            map
        },
    );

    if entries_by_category.is_empty() {
        return Err(AppError::Validation(format!(
            "No builtin roles available for layer '{}'",
            layer.dir_name()
        )));
    }

    let categories: Vec<&str> = entries_by_category.keys().copied().collect();
    loop {
        let mut category_items: Vec<String> =
            categories.iter().map(|value| value.to_string()).collect();
        if allow_layer_back {
            category_items.push(MENU_BACK_OPTION.to_string());
        }

        let category_index = Select::new()
            .with_prompt("Select category")
            .items(&category_items)
            .default(0)
            .interact_opt()
            .map_err(|err| AppError::Validation(format!("Failed to select category: {}", err)))?;

        let Some(category_index) = category_index else {
            return Ok(BuiltinRoleSelection::Cancel);
        };

        if allow_layer_back && category_index == category_items.len() - 1 {
            return Ok(BuiltinRoleSelection::BackToLayer);
        }

        let selected_category = categories[category_index];
        let roles = entries_by_category.get(selected_category).unwrap();

        let mut role_items: Vec<String> = roles
            .iter()
            .map(|entry| format!("{}: {}", entry.name.as_str(), entry.summary))
            .collect();
        role_items.push(MENU_BACK_OPTION.to_string());

        let role_index = Select::new()
            .with_prompt("Select role")
            .items(&role_items)
            .default(0)
            .interact_opt()
            .map_err(|err| AppError::Validation(format!("Failed to select role: {}", err)))?;

        let Some(role_index) = role_index else {
            return Ok(BuiltinRoleSelection::Cancel);
        };

        if role_index == role_items.len() - 1 {
            continue;
        }

        return Ok(BuiltinRoleSelection::Selected(roles[role_index].name.as_str().to_string()));
    }
}

fn prompt_role_name() -> Result<Option<String>, AppError> {
    match Input::new().with_prompt("Role name").interact_text() {
        Ok(value) => Ok(Some(value)),
        Err(DialoguerError::IO(err)) if err.kind() == ErrorKind::Interrupted => Ok(None),
        Err(err) => Err(AppError::Validation(format!("Failed to read role name: {}", err))),
    }
}
