use std::fmt;

/// The architectural layers for agent roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// Observers: Read source, update notes, emit events (taxonomy, data_arch, qa)
    Observers,
    /// Deciders: Read events/issues, emit issues, delete events (triage)
    Deciders,
    /// Planners: Read issues, emit tasks, delete issues (specifier)
    Planners,
    /// Mergers: Consolidate parallel observer branches (consolidator)
    Mergers,
}

impl Layer {
    /// All available layers in order.
    pub const ALL: [Layer; 4] =
        [Layer::Observers, Layer::Deciders, Layer::Planners, Layer::Mergers];

    /// Directory name for this layer.
    pub fn dir_name(&self) -> &'static str {
        match self {
            Layer::Observers => "observers",
            Layer::Deciders => "deciders",
            Layer::Planners => "planners",
            Layer::Mergers => "mergers",
        }
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Layer::Observers => "Observer",
            Layer::Deciders => "Decider",
            Layer::Planners => "Planner",
            Layer::Mergers => "Merger",
        }
    }

    /// Parse a layer from its directory name.
    pub fn from_dir_name(name: &str) -> Option<Layer> {
        match name.to_lowercase().as_str() {
            "observers" | "observer" => Some(Layer::Observers),
            "deciders" | "decider" => Some(Layer::Deciders),
            "planners" | "planner" => Some(Layer::Planners),
            "mergers" | "merger" => Some(Layer::Mergers),
            _ => None,
        }
    }

    /// Description of this layer's responsibilities.
    pub fn description(&self) -> &'static str {
        match self {
            Layer::Observers => "Read source & notes, emit events. Never write issues.",
            Layer::Deciders => "Read events & issues, emit issues. Delete processed events.",
            Layer::Planners => "Read issues, emit tasks. Delete processed issues.",
            Layer::Mergers => "Consolidate parallel observer branches into consistency branch.",
        }
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
}
