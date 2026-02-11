use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct WorkflowExchangeInspectOutput {
    pub schema_version: u32,
    pub schedule: ScheduleSummary,
    pub events: EventSummary,
    pub requirements: RequirementSummary,
}

#[derive(Debug, Serialize)]
pub struct ScheduleSummary {
    pub version: u32,
    pub enabled: bool,
    pub observers: ScheduleLayerSummary,
}

#[derive(Debug, Serialize)]
pub struct ScheduleLayerSummary {
    pub roles: Vec<RoleSummary>,
}

#[derive(Debug, Serialize)]
pub struct RoleSummary {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct EventSummary {
    pub states: Vec<EventStateSummary>,
    pub pending_files: Vec<String>,
    pub items: Vec<EventItem>,
}

#[derive(Debug, Serialize)]
pub struct EventStateSummary {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct EventItem {
    pub path: String,
    pub state: String,
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct RequirementSummary {
    pub count: usize,
    pub items: Vec<RequirementItem>,
}

#[derive(Debug, Serialize)]
pub struct RequirementItem {
    pub path: String,
    pub label: String,
    pub requires_deep_analysis: bool,
    pub id: String,
    pub source_events: Vec<String>,
}
