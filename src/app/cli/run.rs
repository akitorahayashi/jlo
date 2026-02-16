//! Run command implementation.

use std::path::PathBuf;

use crate::domain::AppError;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum RunLayer {
    /// Run narrator layer (summarizes codebase changes)
    #[clap(visible_alias = "n")]
    Narrator {
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run observers layer (requires role)
    #[clap(visible_alias = "o", alias = "observers")]
    Observer {
        /// Role (persona) to run
        role: String,
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run decider layer (single role)
    #[clap(visible_alias = "d", alias = "deciders")]
    Decider {
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run planner layer (requirement-driven)
    #[clap(visible_alias = "p", alias = "planners")]
    Planner {
        /// Local requirement file path (required)
        requirement: PathBuf,
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run implementer layer (requirement-driven)
    #[clap(visible_alias = "i", alias = "implementers")]
    Implementer {
        /// Local requirement file path (required)
        requirement: PathBuf,
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run innovators layer (requires role)
    #[clap(visible_alias = "x", alias = "innovators")]
    Innovator {
        /// Role (persona) to run
        role: String,
        /// Task selector (expected: create_three_proposals)
        #[arg(long)]
        task: Option<String>,
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run integrator layer (merges implementer branches into one integration PR)
    #[clap(visible_alias = "g", alias = "integrators")]
    Integrator {
        /// Show assembled prompts without executing
        #[arg(long)]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
    },
}

pub fn run_agents(layer: RunLayer) -> Result<(), AppError> {
    use crate::domain::Layer;

    let (target_layer, role, prompt_preview, branch, requirement, mock, task) = match layer {
        RunLayer::Narrator { prompt_preview, branch, mock } => {
            (Layer::Narrator, None, prompt_preview, branch, None, mock, None)
        }
        RunLayer::Observer { role, prompt_preview, branch, mock } => {
            (Layer::Observers, Some(role), prompt_preview, branch, None, mock, None)
        }
        RunLayer::Decider { prompt_preview, branch, mock } => {
            (Layer::Decider, None, prompt_preview, branch, None, mock, None)
        }
        RunLayer::Planner { prompt_preview, branch, requirement, mock } => {
            (Layer::Planner, None, prompt_preview, branch, Some(requirement), mock, None)
        }
        RunLayer::Implementer { prompt_preview, branch, requirement, mock } => {
            (Layer::Implementer, None, prompt_preview, branch, Some(requirement), mock, None)
        }
        RunLayer::Innovator { role, task, prompt_preview, branch, mock } => {
            (Layer::Innovators, Some(role), prompt_preview, branch, None, mock, task)
        }
        RunLayer::Integrator { prompt_preview, branch } => {
            (Layer::Integrator, None, prompt_preview, branch, None, false, None)
        }
    };

    let result =
        crate::app::api::run(target_layer, role, prompt_preview, branch, requirement, mock, task)?;

    if !result.prompt_preview && !result.roles.is_empty() && !result.sessions.is_empty() {
        println!("✅ Created {} Jules session(s)", result.sessions.len());
    }

    Ok(())
}
