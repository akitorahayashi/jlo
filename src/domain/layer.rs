use std::fmt;

/// The architectural layers for agent roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// Narrator: Summarize codebase changes, produce changes feed
    Narrators,
    /// Observers: Read source and changes, emit events (taxonomy, data_arch, consistency, qa)
    Observers,
    /// Deciders: Read events, emit issues, delete events (triage_generic)
    Deciders,
    /// Planners: Read issues requiring deep analysis, expand them in-place (specifier_global)
    Planners,
    /// Implementers: Execute approved tasks, create PRs with code changes (executor_global)
    Implementers,
}

impl Layer {
    /// All available layers in order.
    pub const ALL: [Layer; 5] =
        [Layer::Narrators, Layer::Observers, Layer::Deciders, Layer::Planners, Layer::Implementers];

    /// Directory name for this layer.
    pub fn dir_name(&self) -> &'static str {
        match self {
            Layer::Narrators => "narrators",
            Layer::Observers => "observers",
            Layer::Deciders => "deciders",
            Layer::Planners => "planners",
            Layer::Implementers => "implementers",
        }
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Layer::Narrators => "Narrator",
            Layer::Observers => "Observer",
            Layer::Deciders => "Decider",
            Layer::Planners => "Planner",
            Layer::Implementers => "Implementer",
        }
    }

    /// Parse a layer from its directory name.
    pub fn from_dir_name(name: &str) -> Option<Layer> {
        match name.to_lowercase().as_str() {
            "narrators" | "narrator" => Some(Layer::Narrators),
            "observers" | "observer" => Some(Layer::Observers),
            "deciders" | "decider" => Some(Layer::Deciders),
            "planners" | "planner" => Some(Layer::Planners),
            "implementers" | "implementer" => Some(Layer::Implementers),
            _ => None,
        }
    }

    /// Description of this layer's responsibilities.
    pub fn description(&self) -> &'static str {
        match self {
            Layer::Narrators => "Summarize codebase changes, produce changes feed for observers.",
            Layer::Observers => "Read source and changes, emit events. Never write issues.",
            Layer::Deciders => "Read events, emit issues. Delete processed events.",
            Layer::Planners => "Read issues requiring deep analysis, expand them in-place.",
            Layer::Implementers => "Execute approved tasks, create PRs with code changes.",
        }
    }

    /// Whether this layer has a single, fixed role (no subdirectories).
    ///
    /// Single-role layers (Narrators, Planners, Implementers) have prompt.yml directly
    /// in the layer directory rather than in role subdirectories. They do not support
    /// custom role creation or scheduled role lists.
    pub fn is_single_role(&self) -> bool {
        matches!(self, Layer::Narrators | Layer::Planners | Layer::Implementers)
    }

    /// Whether this layer is issue-driven.
    ///
    /// Issue-driven layers (Planners, Implementers) require a local issue file path.
    /// Narrator is single-role but not issue-driven.
    pub fn is_issue_driven(&self) -> bool {
        matches!(self, Layer::Planners | Layer::Implementers)
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_dir_names_are_lowercase() {
        for layer in Layer::ALL {
            assert_eq!(layer.dir_name(), layer.dir_name().to_lowercase());
        }
    }

    #[test]
    fn layer_from_dir_name_roundtrips() {
        for layer in Layer::ALL {
            assert_eq!(Layer::from_dir_name(layer.dir_name()), Some(layer));
        }
    }

    #[test]
    fn all_layers_have_descriptions() {
        for layer in Layer::ALL {
            assert!(!layer.description().is_empty());
            assert!(!layer.display_name().is_empty());
        }
    }

    #[test]
    fn single_role_layers_include_narrator_planners_implementers() {
        assert!(Layer::Narrators.is_single_role());
        assert!(!Layer::Observers.is_single_role());
        assert!(!Layer::Deciders.is_single_role());
        assert!(Layer::Planners.is_single_role());
        assert!(Layer::Implementers.is_single_role());
    }

    #[test]
    fn issue_driven_layers_are_planners_and_implementers() {
        assert!(!Layer::Narrators.is_issue_driven());
        assert!(!Layer::Observers.is_issue_driven());
        assert!(!Layer::Deciders.is_issue_driven());
        assert!(Layer::Planners.is_issue_driven());
        assert!(Layer::Implementers.is_issue_driven());
    }
}
