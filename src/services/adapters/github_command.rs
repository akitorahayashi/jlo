use crate::domain::AppError;
use crate::ports::GitHubPort;
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct GitHubCommandAdapter;

impl GitHubCommandAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl GitHubPort for GitHubCommandAdapter {
    fn dispatch_workflow(
        &self,
        workflow_name: &str,
        inputs: &[(&str, &str)],
    ) -> Result<(), AppError> {
        // Let's rebuild args to handle ownership properly
        let mut cmd = Command::new("gh");
        cmd.args(["workflow", "run", workflow_name]);

        for (key, val) in inputs {
            cmd.arg("-f").arg(format!("{}={}", key, val));
        }

        let output = cmd.output().map_err(|e| AppError::ExternalToolError {
            tool: "gh".into(),
            error: format!("Failed to execute gh CLI: {}", e),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::ExternalToolError {
                tool: "gh".into(),
                error: format!("Failed to dispatch workflow via gh CLI. Stderr:\n{}", stderr),
            });
        }

        Ok(())
    }
}
