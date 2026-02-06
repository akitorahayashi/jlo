//! Mock execution for workflow validation without Jules API.
//!
//! This module provides mock implementations of agent execution that perform
//! real git and GitHub operations without calling the Jules API. This enables
//! end-to-end validation of the workflow kit.

pub mod config;
pub mod decider;
pub mod identity;
pub mod implementer;
pub mod narrator;
pub mod observer;
pub mod planner;

use std::path::Path;

use crate::app::commands::run::RunOptions;
use crate::app::commands::run::RunResult;
use crate::domain::{AppError, Layer, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

use self::config::{load_mock_config, validate_mock_prerequisites};
use self::decider::execute_mock_deciders;
use self::implementer::execute_mock_implementers;
use self::narrator::execute_mock_narrator;
use self::observer::execute_mock_observers;
use self::planner::execute_mock_planners;

/// Execute in mock mode.
pub fn execute<G, H, W>(
    jules_path: &Path,
    options: &RunOptions,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<RunResult, AppError>
where
    G: GitPort,
    H: GitHubPort,
    W: WorkspaceStore,
{
    // Validate mock prerequisites
    validate_mock_prerequisites(options)?;

    // Load mock configuration from workspace
    let mock_config = load_mock_config(jules_path, options, workspace)?;

    // Execute layer-specific mock behavior
    let output = match options.layer {
        Layer::Narrators => execute_mock_narrator(jules_path, &mock_config, git, github, workspace),
        Layer::Observers => {
            execute_mock_observers(jules_path, options, &mock_config, git, github, workspace)
        }
        Layer::Deciders => {
            execute_mock_deciders(jules_path, options, &mock_config, git, github, workspace)
        }
        Layer::Planners => {
            execute_mock_planners(jules_path, options, &mock_config, git, github, workspace)
        }
        Layer::Implementers => {
            execute_mock_implementers(jules_path, options, &mock_config, git, github, workspace)
        }
    }?;

    // Write outputs
    if std::env::var("GITHUB_OUTPUT").is_ok() {
        write_github_output(&output).map_err(|e| {
            AppError::InternalError(format!("Failed to write GITHUB_OUTPUT: {}", e))
        })?;
    } else {
        print_local(&output);
    }

    Ok(RunResult {
        roles: vec![options.layer.dir_name().to_string()],
        prompt_preview: false,
        sessions: vec![], // No Jules sessions in mock mode
    })
}

/// Write outputs to GITHUB_OUTPUT file if set.
fn write_github_output(output: &MockOutput) -> std::io::Result<()> {
    if let Ok(output_file) = std::env::var("GITHUB_OUTPUT") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(&output_file)?;
        writeln!(file, "mock_branch={}", output.mock_branch)?;
        writeln!(file, "mock_pr_number={}", output.mock_pr_number)?;
        writeln!(file, "mock_pr_url={}", output.mock_pr_url)?;
        writeln!(file, "mock_tag={}", output.mock_tag)?;
    }
    Ok(())
}

/// Print outputs in grep-friendly format for local use.
fn print_local(output: &MockOutput) {
    println!("MOCK_BRANCH={}", output.mock_branch);
    println!("MOCK_PR_NUMBER={}", output.mock_pr_number);
    println!("MOCK_PR_URL={}", output.mock_pr_url);
    println!("MOCK_TAG={}", output.mock_tag);
}
