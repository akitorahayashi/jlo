use std::process::Command;
use std::time::Duration;

use crate::domain::AppError;
use crate::ports::{GitHubPort, PullRequestInfo};

#[derive(Debug, Clone, Default)]
pub struct GitHubCommandAdapter;

impl GitHubCommandAdapter {
    pub fn new() -> Self {
        Self
    }

    fn run_gh(&self, args: &[&str]) -> Result<String, AppError> {
        let mut cmd = Command::new("gh");
        cmd.args(args);

        let output = cmd.output().map_err(|e| AppError::ExternalToolError {
            tool: "gh".into(),
            error: format!("Failed to execute gh CLI: {}", e),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::ExternalToolError {
                tool: "gh".into(),
                error: format!("gh command failed: {}", stderr.trim()),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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

    fn create_pull_request(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> Result<PullRequestInfo, AppError> {
        // Create PR and get JSON output
        let output = self.run_gh(&[
            "pr",
            "create",
            "--head",
            head,
            "--base",
            base,
            "--title",
            title,
            "--body",
            body,
            "--json",
            "number,url,headRefName,baseRefName",
        ])?;

        // Parse JSON response
        let json: serde_json::Value = serde_json::from_str(&output).map_err(|e| {
            AppError::ParseError { what: "PR JSON response".into(), details: e.to_string() }
        })?;

        Ok(PullRequestInfo {
            number: json["number"].as_u64().unwrap_or(0),
            url: json["url"].as_str().unwrap_or("").to_string(),
            head: json["headRefName"].as_str().unwrap_or(head).to_string(),
            base: json["baseRefName"].as_str().unwrap_or(base).to_string(),
        })
    }

    fn wait_for_merge(&self, pr_number: u64, timeout: Duration) -> Result<(), AppError> {
        let start = std::time::Instant::now();
        let pr_num_str = pr_number.to_string();

        while start.elapsed() < timeout {
            let output = self.run_gh(&["pr", "view", &pr_num_str, "--json", "state,mergedAt"])?;

            let json: serde_json::Value = serde_json::from_str(&output).map_err(|e| {
                AppError::ParseError { what: "PR state JSON".into(), details: e.to_string() }
            })?;

            let state = json["state"].as_str().unwrap_or("");
            if state == "MERGED" || json["mergedAt"].as_str().is_some() {
                return Ok(());
            }

            if state == "CLOSED" {
                return Err(AppError::Validation(format!(
                    "PR #{} was closed without merging",
                    pr_number
                )));
            }

            std::thread::sleep(Duration::from_secs(5));
        }

        Err(AppError::Validation(format!(
            "Timeout waiting for PR #{} to merge after {:?}",
            pr_number, timeout
        )))
    }

    fn close_pull_request(&self, pr_number: u64) -> Result<(), AppError> {
        let pr_num_str = pr_number.to_string();
        self.run_gh(&["pr", "close", &pr_num_str])?;
        Ok(())
    }

    fn delete_branch(&self, branch: &str) -> Result<(), AppError> {
        // Use gh api to delete branch
        let endpoint = format!("repos/{{owner}}/{{repo}}/git/refs/heads/{}", branch);
        self.run_gh(&["api", "-X", "DELETE", &endpoint])?;
        Ok(())
    }

    fn enable_auto_merge(&self, pr_number: u64) -> Result<(), AppError> {
        let pr_num_str = pr_number.to_string();
        self.run_gh(&["pr", "merge", &pr_num_str, "--auto", "--squash"])?;
        Ok(())
    }
}
