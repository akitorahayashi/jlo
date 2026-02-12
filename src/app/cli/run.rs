//! Run command implementation.

use std::path::PathBuf;

use crate::domain::AppError;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum RunLayer {
    /// Run narrator agent (summarizes codebase changes)
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
    /// Run observer agent (single role)
    #[clap(visible_alias = "o")]
    Observers {
        /// Role to run
        #[arg(short = 'r', long)]
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
    /// Run decider agent (single role)
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
    /// Run planner agent (single-role, requirement-driven)
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
    /// Run implementer agent (single-role, requirement-driven)
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
    /// Run innovator agent (single role)
    #[clap(visible_alias = "x")]
    Innovators {
        /// Role (persona) to run
        #[arg(short = 'r', long)]
        role: String,
        /// Execution phase (creation or refinement)
        #[arg(long)]
        phase: Option<String>,
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
}

pub fn run_agents(layer: RunLayer) -> Result<(), AppError> {
    use crate::domain::Layer;

    let (target_layer, role, prompt_preview, branch, requirement, mock, phase) = match layer {
        RunLayer::Narrator { prompt_preview, branch, mock } => {
            (Layer::Narrator, None, prompt_preview, branch, None, mock, None)
        }
        RunLayer::Observers { role, prompt_preview, branch, mock } => {
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
        RunLayer::Innovators { role, phase, prompt_preview, branch, mock } => {
            (Layer::Innovators, Some(role), prompt_preview, branch, None, mock, phase)
        }
    };

    let result =
        crate::app::api::run(target_layer, role, prompt_preview, branch, requirement, mock, phase)?;

    if !result.prompt_preview && !result.roles.is_empty() && !result.sessions.is_empty() {
        println!("✅ Created {} Jules session(s)", result.sessions.len());
    }

    Ok(())
}
