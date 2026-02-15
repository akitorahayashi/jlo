/// Workflow generate configuration for template expansion.
///
/// Values are sourced from `.jlo/config.toml` and rendered
/// as static literals in generated workflow YAML.
#[derive(Debug, Clone)]
pub struct WorkflowGenerateConfig {
    /// Target/control branch (e.g. `main`). Maps to `run.jlo_target_branch`.
    pub target_branch: String,
    /// Worker branch hosting `.jules/` runtime state. Maps to `run.jules_worker_branch`.
    pub worker_branch: String,
    /// Cron entries used for workflow schedule.
    pub schedule_crons: Vec<String>,
    /// Default wait minutes for orchestration pacing.
    pub wait_minutes_default: u32,
}

impl Default for WorkflowGenerateConfig {
    fn default() -> Self {
        Self {
            target_branch: "main".to_string(),
            worker_branch: "jules".to_string(),
            schedule_crons: vec!["0 20 * * *".to_string()],
            wait_minutes_default: 30,
        }
    }
}
