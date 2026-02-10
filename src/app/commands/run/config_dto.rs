use serde::Deserialize;
use url::Url;

use crate::domain::{ExecutionConfig, JulesApiConfig, RunConfig};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunConfigDto {
    pub run: Option<ExecutionConfigDto>,
    pub jules: Option<JulesApiConfigDto>,
    #[allow(dead_code)]
    pub workflow: Option<WorkflowTimingConfigDto>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionConfigDto {
    pub default_branch: Option<String>,
    pub jules_branch: Option<String>,
    pub parallel: Option<bool>,
    pub max_parallel: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JulesApiConfigDto {
    pub api_url: Option<Url>,
    pub timeout_secs: Option<u64>,
    pub max_retries: Option<u32>,
    pub retry_delay_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct WorkflowTimingConfigDto {
    pub runner_mode: Option<String>,
    pub cron: Option<Vec<String>>,
    pub wait_minutes_default: Option<u32>,
}

impl From<RunConfigDto> for RunConfig {
    fn from(dto: RunConfigDto) -> Self {
        let default_run = ExecutionConfig::default();
        let run = if let Some(d) = dto.run {
            ExecutionConfig {
                default_branch: d.default_branch.unwrap_or(default_run.default_branch),
                jules_branch: d.jules_branch.unwrap_or(default_run.jules_branch),
                parallel: d.parallel.unwrap_or(default_run.parallel),
                max_parallel: d.max_parallel.unwrap_or(default_run.max_parallel),
            }
        } else {
            default_run
        };

        let default_jules = JulesApiConfig::default();
        let jules = if let Some(d) = dto.jules {
            JulesApiConfig {
                api_url: d.api_url.unwrap_or(default_jules.api_url),
                timeout_secs: d.timeout_secs.unwrap_or(default_jules.timeout_secs),
                max_retries: d.max_retries.unwrap_or(default_jules.max_retries),
                retry_delay_ms: d.retry_delay_ms.unwrap_or(default_jules.retry_delay_ms),
            }
        } else {
            default_jules
        };

        RunConfig { run, jules }
    }
}
