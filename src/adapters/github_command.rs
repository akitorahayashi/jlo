use std::io::Write;
use std::process::{Command, Stdio};

use crate::domain::AppError;
use crate::ports::{GitHubPort, IssueInfo, PrComment, PullRequestDetail, PullRequestInfo};

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

    fn run_gh_with_input(&self, args: &[&str], input: &str) -> Result<String, AppError> {
        let mut cmd = Command::new("gh");
        cmd.args(args).stdin(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| AppError::ExternalToolError {
            tool: "gh".into(),
            error: format!("Failed to execute gh CLI: {}", e),
        })?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(input.as_bytes()).map_err(|e| AppError::ExternalToolError {
                tool: "gh".into(),
                error: format!("Failed to write gh CLI input: {}", e),
            })?;
        }

        let output = child.wait_with_output().map_err(|e| AppError::ExternalToolError {
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

    fn get_pr_detail(&self, pr_number: u64) -> Result<PullRequestDetail, AppError> {
        let pr_num_str = pr_number.to_string();
        let output = self.run_gh(&[
            "pr",
            "view",
            &pr_num_str,
            "--json",
            "number,headRefName,baseRefName,isDraft,autoMergeRequest",
        ])?;
        let json: serde_json::Value =
            serde_json::from_str(&output).map_err(|e| AppError::ParseError {
                what: "PR detail JSON".into(),
                details: format!("Failed to parse gh pr view output: {}", e),
            })?;
        Ok(PullRequestDetail {
            number: json["number"].as_u64().unwrap_or(pr_number),
            head: json["headRefName"].as_str().unwrap_or_default().to_string(),
            base: json["baseRefName"].as_str().unwrap_or_default().to_string(),
            is_draft: json["isDraft"].as_bool().unwrap_or(false),
            auto_merge_enabled: !json["autoMergeRequest"].is_null(),
        })
    }

    fn list_pr_comments(&self, pr_number: u64) -> Result<Vec<PrComment>, AppError> {
        let pr_num_str = pr_number.to_string();
        // Use gh api to list issue comments on a PR
        let endpoint =
            format!("repos/{{owner}}/{{repo}}/issues/{}/comments?per_page=100", pr_num_str);
        let output = self.run_gh(&["api", "--paginate", &endpoint])?;
        let json: Vec<serde_json::Value> =
            serde_json::from_str(&output).map_err(|e| AppError::ParseError {
                what: "PR comments JSON".into(),
                details: format!("Failed to parse gh api comments: {}", e),
            })?;
        let comments = json
            .into_iter()
            .filter_map(|c| {
                let id = c["id"].as_u64()?;
                let body = c["body"].as_str()?.to_string();
                Some(PrComment { id, body })
            })
            .collect();
        Ok(comments)
    }

    fn create_pr_comment(&self, pr_number: u64, body: &str) -> Result<u64, AppError> {
        let endpoint = format!("repos/{{owner}}/{{repo}}/issues/{}/comments", pr_number);
        let payload = serde_json::json!({ "body": body }).to_string();
        let output =
            self.run_gh_with_input(&["api", "-X", "POST", &endpoint, "--input", "-"], &payload)?;
        let json: serde_json::Value =
            serde_json::from_str(&output).map_err(|e| AppError::ParseError {
                what: "PR comment creation response".into(),
                details: format!("Failed to parse gh api response: {}", e),
            })?;
        json["id"].as_u64().ok_or_else(|| {
            AppError::InternalError("Created PR comment but response missing id field".into())
        })
    }

    fn update_pr_comment(&self, comment_id: u64, body: &str) -> Result<(), AppError> {
        let endpoint = format!("repos/{{owner}}/{{repo}}/issues/comments/{}", comment_id);
        let payload = serde_json::json!({ "body": body }).to_string();
        self.run_gh_with_input(&["api", "-X", "PATCH", &endpoint, "--input", "-"], &payload)?;
        Ok(())
    }

    fn ensure_label(&self, label: &str, color: Option<&str>) -> Result<(), AppError> {
        // Check if label exists
        let list_output = self.run_gh(&["label", "list", "--json", "name", "-q", ".[].name"])?;
        let label_exists = list_output.lines().any(|l| l == label);

        if label_exists {
            // Label already exists — nothing to do
            Ok(())
        } else if let Some(c) = color {
            self.run_gh(&["label", "create", label, "--color", c, "--force"])?;
            Ok(())
        } else {
            // Create without color → GitHub assigns random color
            self.run_gh(&["label", "create", label, "--force"])?;
            Ok(())
        }
    }

    fn add_label_to_pr(&self, pr_number: u64, label: &str) -> Result<(), AppError> {
        let pr_num_str = pr_number.to_string();
        self.run_gh(&["pr", "edit", &pr_num_str, "--add-label", label])?;
        Ok(())
    }

    fn add_label_to_issue(&self, issue_number: u64, label: &str) -> Result<(), AppError> {
        let issue_num_str = issue_number.to_string();
        self.run_gh(&["issue", "edit", &issue_num_str, "--add-label", label])?;
        Ok(())
    }

    fn enable_automerge(&self, pr_number: u64) -> Result<(), AppError> {
        let pr_num_str = pr_number.to_string();
        self.run_gh(&["pr", "merge", &pr_num_str, "--auto", "--squash", "--delete-branch"])?;
        Ok(())
    }

    fn list_pr_files(&self, pr_number: u64) -> Result<Vec<String>, AppError> {
        let pr_num_str = pr_number.to_string();
        let output = self.run_gh(&["pr", "diff", &pr_num_str, "--name-only"])?;
        let files = output.lines().map(|l| l.to_string()).collect();
        Ok(files)
    }
}
