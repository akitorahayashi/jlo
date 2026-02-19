/// Builder for requirement YAML payloads used in domain and app unit tests.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RequirementYamlBuilder {
    label: String,
    implementation_ready: bool,
}

impl Default for RequirementYamlBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl RequirementYamlBuilder {
    pub fn new() -> Self {
        Self { label: "bugs".to_string(), implementation_ready: true }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn implementation_ready(mut self, ready: bool) -> Self {
        self.implementation_ready = ready;
        self
    }

    pub fn build(self) -> String {
        format!("label: {}\nimplementation_ready: {}\n", self.label, self.implementation_ready)
    }
}
