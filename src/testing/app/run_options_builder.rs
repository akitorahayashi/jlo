use std::path::PathBuf;

use crate::domain::{Layer, RunOptions};

/// Builder for `RunOptions` used by app-layer unit tests.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RunOptionsBuilder {
    layer: Layer,
    role: Option<String>,
    prompt_preview: bool,
    branch: Option<String>,
    requirement: Option<PathBuf>,
    mock: bool,
    task: Option<String>,
}

#[allow(dead_code)]
impl RunOptionsBuilder {
    pub fn for_layer(layer: Layer) -> Self {
        Self {
            layer,
            role: None,
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: false,
            task: None,
        }
    }

    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    pub fn prompt_preview(mut self, enabled: bool) -> Self {
        self.prompt_preview = enabled;
        self
    }

    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    pub fn requirement(mut self, requirement: impl Into<PathBuf>) -> Self {
        self.requirement = Some(requirement.into());
        self
    }

    pub fn mock(mut self, enabled: bool) -> Self {
        self.mock = enabled;
        self
    }

    pub fn task(mut self, task: impl Into<String>) -> Self {
        self.task = Some(task.into());
        self
    }

    pub fn build(self) -> RunOptions {
        RunOptions {
            layer: self.layer,
            role: self.role,
            prompt_preview: self.prompt_preview,
            branch: self.branch,
            requirement: self.requirement,
            mock: self.mock,
            task: self.task,
        }
    }
}
