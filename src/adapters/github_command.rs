use std::process::Command;

use crate::domain::AppError;
use crate::ports::{GitHubPort, IssueInfo, PullRequestInfo};

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
        // Create PR
        let output = self.run_gh(&[
            "pr", "create", "--head", head, "--base", base, "--title", title, "--body", body,
        ])?;

        // Extract PR URL from output (gh pr create prints the URL on success)
        let url = output.trim();
        if !url.starts_with("https://") {
            return Err(AppError::ExternalToolError {
                tool: "gh".into(),
                error: format!("Unexpected output from gh pr create: {}", output),
            });
        }

        // Extract PR number from URL (format: https://github.com/owner/repo/pull/123)
        let pr_number =
            url.split('/').next_back().and_then(|s| s.parse::<u64>().ok()).ok_or_else(|| {
                AppError::ParseError {
                    what: "PR URL".into(),
                    details: format!("Could not extract PR number from URL: {}", url),
                }
            })?;

        Ok(PullRequestInfo {
            number: pr_number,
            url: url.to_string(),
            head: head.to_string(),
            base: base.to_string(),
        })
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

    fn create_issue(
        &self,
        title: &str,
        body: &str,
        labels: &[&str],
    ) -> Result<IssueInfo, AppError> {
        let mut args = vec!["issue", "create", "--title", title, "--body", body];
        let labels_csv = labels.join(",");
        if !labels.is_empty() {
            args.push("--label");
            args.push(&labels_csv);
        }

        let output = self.run_gh(&args)?;

        // gh issue create prints the issue URL on success
        let url = output.trim();
        if !url.starts_with("https://") {
            return Err(AppError::ExternalToolError {
                tool: "gh".into(),
                error: format!("Unexpected output from gh issue create: {}", output),
            });
        }

        // Extract issue number from URL (format: https://github.com/owner/repo/issues/123)
        let issue_number =
            url.split('/').next_back().and_then(|s| s.parse::<u64>().ok()).ok_or_else(|| {
                AppError::ParseError {
                    what: "issue URL".into(),
                    details: format!("Could not extract issue number from URL: {}", url),
                }
            })?;

        Ok(IssueInfo { number: issue_number, url: url.to_string() })
    }
}
