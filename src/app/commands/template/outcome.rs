use crate::domain::Layer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateOutcome {
    Role { layer: Layer, role: String },
    Workstream { name: String },
}

impl TemplateOutcome {
    pub fn display_path(&self) -> String {
        match self {
            TemplateOutcome::Role { layer, role } => {
                format!(".jules/roles/{}/{}", layer.dir_name(), role)
            }
            TemplateOutcome::Workstream { name } => {
                format!(".jules/workstreams/{}", name)
            }
        }
    }
}
