/// Builder for requirement YAML payloads used in domain and app unit tests.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RequirementYamlBuilder {
    label: String,
    implementation_ready: bool,
    planner_request_reason: String,
}

impl Default for RequirementYamlBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl RequirementYamlBuilder {
    pub fn new() -> Self {
        Self {
            label: "bugs".to_string(),
            implementation_ready: true,
            planner_request_reason: String::new(),
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn implementation_ready(mut self, ready: bool) -> Self {
        self.implementation_ready = ready;
        self
    }

    pub fn planner_request_reason(mut self, reason: impl Into<String>) -> Self {
        self.planner_request_reason = reason.into();
        self
    }

    pub fn build(self) -> String {
        let planner_request_reason = if self.implementation_ready {
            String::new()
        } else if self.planner_request_reason.trim().is_empty() {
            "Planner elaboration required".to_string()
        } else {
            self.planner_request_reason
        };

        format!(
            "label: {}\nimplementation_ready: {}\nplanner_request_reason: {}\n",
            self.label, self.implementation_ready, planner_request_reason
        )
    }
}
