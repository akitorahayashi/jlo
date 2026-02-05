//! Prompt assembly for Jules agents.
//!
//! This module provides a unified interface for prompt assembly, delegating
//! to the prompt_assembly service for asset-driven prompt composition.

use std::io;
use std::path::Path;

use crate::domain::{AppError, Layer, PromptContext};
use crate::ports::WorkspaceStore;
use crate::services::application::prompt_assembly::{self, PromptFs, RealPromptFs};

struct WorkspacePromptFs<'a, W: WorkspaceStore> {
    workspace: &'a W,
}

impl<'a, W: WorkspaceStore> PromptFs for WorkspacePromptFs<'a, W> {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let path_str = path.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Invalid unicode in path")
        })?;
        self.workspace
            .read_file(path_str)
            .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))
    }

    fn exists(&self, path: &Path) -> bool {
        let path_str = match path.to_str() {
            Some(s) => s,
            None => return false,
        };
        // Inefficient but compatible check
        self.workspace.read_file(path_str).is_ok()
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        let path_str = path.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Invalid unicode in path")
        })?;
        self.workspace
            .create_dir_all(path_str)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        let from_str = from.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Invalid unicode in from path")
        })?;
        let to_str = to.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Invalid unicode in to path")
        })?;
        self.workspace
            .copy_file(from_str, to_str)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }
}

/// Assemble the full prompt for a role in a multi-role layer.
///
/// Multi-role layers (observers, deciders) require workstream and role context.
pub fn assemble_prompt(
    jules_path: &Path,
    layer: Layer,
    role: &str,
    workstream: &str,
) -> Result<String, AppError> {
    let context = PromptContext::new().with_var("workstream", workstream).with_var("role", role);
    let fs = RealPromptFs;

    Ok(prompt_assembly::assemble_prompt(jules_path, layer, &context, &fs)?.content)
}

/// Assemble the prompt for a single-role layer (Narrator, Planners, Implementers).
///
/// Single-role layers have prompt.yml directly in the layer directory,
/// not in a role subdirectory. They do not require workstream/role context.
pub fn assemble_single_role_prompt(
    jules_path: &Path,
    layer: Layer,
    workspace: &impl WorkspaceStore,
) -> Result<String, AppError> {
    let fs = WorkspacePromptFs { workspace };
    Ok(prompt_assembly::assemble_prompt(jules_path, layer, &PromptContext::new(), &fs)?.content)
}

/// Assemble the prompt for an issue-driven layer with embedded issue content.
///
/// This is used for planners and implementers where the issue content is
/// appended to the base prompt.
#[allow(dead_code)]
pub fn assemble_issue_prompt(
    jules_path: &Path,
    layer: Layer,
    issue_content: &str,
) -> Result<String, AppError> {
    let fs = RealPromptFs;
    Ok(prompt_assembly::assemble_with_issue(jules_path, layer, issue_content, &fs)?.content)
}
