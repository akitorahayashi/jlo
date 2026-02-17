pub mod execute;
pub mod paths;
pub mod prompt_assemble;

use serde::Serialize;
use std::fmt;

/// The architectural layers for execution roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// Narrator: Summarize codebase changes, produce changes feed
    Narrator,
    /// Observers: Read source and changes, emit events (taxonomy, data_arch, consistency, qa)
    Observers,
    /// Decider: Read events, emit issues, delete events
    Decider,
    /// Planner: Read issues requiring deep analysis, expand them in-place (specifier_global)
    Planner,
    /// Implementer: Execute approved tasks, create PRs with code changes (executor_global)
    Implementer,
    /// Innovators: Generate improvement proposals from role workstations
    Innovators,
    /// Integrator: Merge all implementer branches into one integration branch
    Integrator,
}

impl Layer {
    /// All available layers in order.
    pub const ALL: [Layer; 7] = [
        Layer::Narrator,
        Layer::Observers,
        Layer::Decider,
        Layer::Planner,
        Layer::Implementer,
        Layer::Innovators,
        Layer::Integrator,
    ];

    /// Directory name for this layer.
    pub fn dir_name(&self) -> &'static str {
        match self {
            Layer::Narrator => "narrator",
            Layer::Observers => "observers",
            Layer::Decider => "decider",
            Layer::Planner => "planner",
            Layer::Implementer => "implementer",
            Layer::Innovators => "innovators",
            Layer::Integrator => "integrator",
        }
    }

    /// Filename of the Jinja2 prompt template for this layer.
    ///
    /// Observers and Innovators use plural forms (multi-role layers).
    /// All other layers use singular.
    pub fn prompt_template_name(&self) -> &'static str {
        match self {
            Layer::Narrator => "narrator_prompt.j2",
            Layer::Observers => "observers_prompt.j2",
            Layer::Decider => "decider_prompt.j2",
            Layer::Planner => "planner_prompt.j2",
            Layer::Implementer => "implementer_prompt.j2",
            Layer::Innovators => "innovators_prompt.j2",
            Layer::Integrator => "integrator_prompt.j2",
        }
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Layer::Narrator => "Narrator",
            Layer::Observers => "Observers",
            Layer::Decider => "Decider",
            Layer::Planner => "Planner",
            Layer::Implementer => "Implementer",
            Layer::Innovators => "Innovators",
            Layer::Integrator => "Integrator",
        }
    }

    /// Parse a layer from its directory name.
    pub fn from_dir_name(name: &str) -> Option<Layer> {
        match name.to_lowercase().as_str() {
            "narrator" | "narrators" => Some(Layer::Narrator),
            "observers" | "observer" | "o" => Some(Layer::Observers),
            "deciders" | "decider" => Some(Layer::Decider),
            "planners" | "planner" => Some(Layer::Planner),
            "implementers" | "implementer" | "i" => Some(Layer::Implementer),
            "innovators" | "innovator" | "x" => Some(Layer::Innovators),
            "integrator" | "integrators" => Some(Layer::Integrator),
            _ => None,
        }
    }

    /// Description of this layer's responsibilities.
    pub fn description(&self) -> &'static str {
        match self {
            Layer::Narrator => "Summarize codebase changes, produce changes feed for observers.",
            Layer::Observers => "Read source and changes, emit events. Never write issues.",
            Layer::Decider => "Read events, emit issues. Delete processed events.",
            Layer::Planner => "Read issues requiring deep analysis, expand them in-place.",
            Layer::Implementer => "Execute approved tasks, create PRs with code changes.",
            Layer::Innovators => {
                "Generate improvement proposals from repository context and role workstations."
            }
            Layer::Integrator => "Merge all implementer branches into one integration branch.",
        }
    }

    /// Whether this layer has a single, fixed role (no subdirectories).
    ///
    /// Single-role layers (Narrator, Decider, Planner, Implementer) have contracts.yml
    /// directly in the layer directory rather than in role subdirectories. They do not
    /// support custom role creation or scheduled role lists.
    pub fn is_single_role(&self) -> bool {
        matches!(
            self,
            Layer::Narrator
                | Layer::Decider
                | Layer::Planner
                | Layer::Implementer
                | Layer::Integrator
        )
    }

    /// Whether this layer emits proposal artifacts.
    pub fn is_innovator(&self) -> bool {
        matches!(self, Layer::Innovators)
    }

    /// Whether this layer executes on the worker branch.
    ///
    /// Layers that operate on the `.jules/` runtime repository (narrator, observers,
    /// decider, planner, innovators) use the worker branch. Layers that operate on
    /// production code (implementer, integrator) use the target branch.
    pub fn uses_worker_branch(&self) -> bool {
        match self {
            Layer::Narrator
            | Layer::Observers
            | Layer::Decider
            | Layer::Planner
            | Layer::Innovators => true,
            Layer::Implementer | Layer::Integrator => false,
        }
    }

    /// Whether this layer is issue-driven.
    ///
    /// Issue-driven layers (Planner, Implementer) require a local issue file path.
    /// Narrator is single-role but not issue-driven.
    pub fn is_issue_driven(&self) -> bool {
        matches!(self, Layer::Planner | Layer::Implementer)
    }

    /// Returns the YAML key used for the role identifier in a workstation perspective file.
    pub fn perspective_role_key(&self) -> Result<&'static str, crate::domain::AppError> {
        match self {
            Layer::Innovators => Ok("role"),
            Layer::Observers => Ok("observer"),
            _ => Err(crate::domain::AppError::RepositoryIntegrity(format!(
                "Unsupported layer for workstation perspective materialization: '{}'",
                self.dir_name()
            ))),
        }
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl Serialize for Layer {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.dir_name())
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
    fn single_role_layers_include_narrator_planner_implementer_integrator() {
        assert!(Layer::Narrator.is_single_role());
        assert!(!Layer::Observers.is_single_role());
        assert!(Layer::Decider.is_single_role());
        assert!(Layer::Planner.is_single_role());
        assert!(Layer::Implementer.is_single_role());
        assert!(!Layer::Innovators.is_single_role());
        assert!(Layer::Integrator.is_single_role());
    }

    #[test]
    fn issue_driven_layers_are_planner_and_implementer() {
        assert!(!Layer::Narrator.is_issue_driven());
        assert!(!Layer::Observers.is_issue_driven());
        assert!(!Layer::Decider.is_issue_driven());
        assert!(Layer::Planner.is_issue_driven());
        assert!(Layer::Implementer.is_issue_driven());
        assert!(!Layer::Innovators.is_issue_driven());
        assert!(!Layer::Integrator.is_issue_driven());
    }

    #[test]
    fn innovator_layer_is_identified() {
        assert!(!Layer::Narrator.is_innovator());
        assert!(!Layer::Observers.is_innovator());
        assert!(!Layer::Decider.is_innovator());
        assert!(!Layer::Planner.is_innovator());
        assert!(!Layer::Implementer.is_innovator());
        assert!(Layer::Innovators.is_innovator());
        assert!(!Layer::Integrator.is_innovator());
    }

    #[test]
    fn perspective_role_key_for_innovators_is_role() {
        assert_eq!(Layer::Innovators.perspective_role_key().unwrap(), "role");
    }

    #[test]
    fn uses_worker_branch_matches_branch_contract() {
        assert!(Layer::Narrator.uses_worker_branch());
        assert!(Layer::Observers.uses_worker_branch());
        assert!(Layer::Decider.uses_worker_branch());
        assert!(Layer::Planner.uses_worker_branch());
        assert!(Layer::Innovators.uses_worker_branch());
        assert!(!Layer::Implementer.uses_worker_branch());
        assert!(!Layer::Integrator.uses_worker_branch());
    }

    #[test]
    fn layer_aliases_match_cli() {
        // "i" should map to Implementer, "x" should map to Innovators
        assert_eq!(Layer::from_dir_name("i"), Some(Layer::Implementer));
        assert_eq!(Layer::from_dir_name("x"), Some(Layer::Innovators));
    }
}
