/// Builder for requirement YAML payloads used in domain and app unit tests.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RequirementYamlBuilder {
    label: String,
    requires_deep_analysis: bool,
}

impl Default for RequirementYamlBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl RequirementYamlBuilder {
    pub fn new() -> Self {
        Self { label: "bugs".to_string(), requires_deep_analysis: false }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn requires_deep_analysis(mut self, required: bool) -> Self {
        self.requires_deep_analysis = required;
        self
    }

    pub fn build(self) -> String {
        format!("label: {}\nrequires_deep_analysis: {}\n", self.label, self.requires_deep_analysis)
    }
}
