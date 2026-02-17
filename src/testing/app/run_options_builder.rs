use std::path::PathBuf;

use crate::app::commands::run::RunRuntimeOptions;
use crate::domain::{Layer, RunOptions};

/// Builder for `RunOptions` used by app-layer unit tests.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RunOptionsBuilder {
    layer: Layer,
    role: Option<String>,
    requirement: Option<PathBuf>,
    task: Option<String>,
    runtime: RunRuntimeOptions,
}

#[allow(dead_code)]
impl RunOptionsBuilder {
    pub fn for_layer(layer: Layer) -> Self {
        Self {
            layer,
            role: None,
            requirement: None,
            task: None,
            runtime: RunRuntimeOptions::default(),
        }
    }

    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    pub fn prompt_preview(mut self, enabled: bool) -> Self {
        self.runtime.prompt_preview = enabled;
        self
    }

    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.runtime.branch = Some(branch.into());
        self
    }

    pub fn requirement(mut self, requirement: impl Into<PathBuf>) -> Self {
        self.requirement = Some(requirement.into());
        self
    }

    pub fn mock(mut self, enabled: bool) -> Self {
        self.runtime.mock = enabled;
        self
    }

    pub fn task(mut self, task: impl Into<String>) -> Self {
        self.task = Some(task.into());
        self
    }

    pub fn no_cleanup(mut self, enabled: bool) -> Self {
        self.runtime.no_cleanup = enabled;
        self
    }

    pub fn build(self) -> RunOptions {
        RunOptions {
            layer: self.layer,
            role: self.role,
            requirement: self.requirement,
            task: self.task,
        }
    }

    pub fn build_with_runtime(self) -> (RunOptions, RunRuntimeOptions) {
        let target = RunOptions {
            layer: self.layer,
            role: self.role,
            requirement: self.requirement,
            task: self.task,
        };
        (target, self.runtime)
    }
}
